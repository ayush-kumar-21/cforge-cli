use crate::config::Config;
use crate::platform::{
    self, command_exists, ensure_cmake, ensure_ninja, ensure_pkg_manager, ensure_vcpkg,
    install_pkg, os, vcpkg_exe, vcpkg_root, vcpkg_toolchain_file, vcpkg_triplet, Os, PkgManager,
};
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct TargetFile {
    pub path: std::path::PathBuf,
    pub suffix: &'static str,
}

#[derive(Debug)]
pub enum ResolvedTarget {
    Found(TargetFile),
    NotFound,
    Ambiguous(Vec<std::path::PathBuf>),
}

/// Checks `<base>/C/<name>.c`, `<base>/CPP/<name>.cpp`,
/// and (macOS/Linux only) `<base>/Obj_C/<name>.m`, `<base>/Obj_CPP/<name>.mm`.
/// Takes `base` as a parameter (rather than always using ".") so tests can
/// point it at a scratch directory instead of racing on process cwd.
///
/// If `name` itself carries one of those extensions (e.g. "creditCard.c"),
/// that's a direct pointer to a single file/language — checked on its own
/// without touching the other directories, so it can never come back
/// Ambiguous. This is the only way to build/run one half of two files that
/// share a base name across languages (`C/creditCard.c` and
/// `CPP/creditCard.cpp`), since the bare name is genuinely ambiguous
/// between them.
pub fn resolve_target_in(base: &Path, name: &str) -> ResolvedTarget {
    let name_path = Path::new(name);
    if let (Some(stem), Some(ext)) =
        (name_path.file_stem().and_then(|s| s.to_str()), name_path.extension().and_then(|e| e.to_str()))
    {
        let qualified: Option<(&str, &'static str)> = match ext {
            "c" => Some(("C", "c")),
            "cpp" => Some(("CPP", "cpp")),
            "m" if crate::project::objc_capable_platform() => Some(("Obj_C", "objc")),
            "mm" if crate::project::objc_capable_platform() => Some(("Obj_CPP", "objcpp")),
            _ => None,
        };
        if let Some((dir, suffix)) = qualified {
            let path = base.join(dir).join(format!("{stem}.{ext}"));
            return if path.is_file() { ResolvedTarget::Found(TargetFile { path, suffix }) } else { ResolvedTarget::NotFound };
        }
    }

    let mut candidates = vec![
        (base.join("C").join(format!("{name}.c")), "c"),
        (base.join("CPP").join(format!("{name}.cpp")), "cpp"),
    ];
    if crate::project::objc_capable_platform() {
        candidates.push((base.join("Obj_C").join(format!("{name}.m")), "objc"));
        candidates.push((base.join("Obj_CPP").join(format!("{name}.mm")), "objcpp"));
    }

    let found: Vec<(std::path::PathBuf, &'static str)> =
        candidates.into_iter().filter(|(p, _)| p.is_file()).collect();

    match found.len() {
        0 => ResolvedTarget::NotFound,
        1 => {
            let (path, suffix) = found.into_iter().next().unwrap();
            ResolvedTarget::Found(TargetFile { path, suffix })
        }
        _ => ResolvedTarget::Ambiguous(found.into_iter().map(|(p, _)| p).collect()),
    }
}

pub fn resolve_target(name: &str) -> ResolvedTarget {
    resolve_target_in(Path::new("."), name)
}

#[cfg(test)]
mod resolve_tests {
    use super::*;
    use std::fs;

    fn scratch_dir(label: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("cforge_test_{label}_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn finds_single_match() {
        let base = scratch_dir("single");
        fs::create_dir_all(base.join("C")).unwrap();
        fs::write(base.join("C").join("foo.c"), "").unwrap();
        match resolve_target_in(&base, "foo") {
            ResolvedTarget::Found(t) => assert_eq!(t.suffix, "c"),
            other => panic!("expected Found, got {other:?}"),
        }
        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn reports_ambiguous_matches() {
        let base = scratch_dir("ambiguous");
        fs::create_dir_all(base.join("C")).unwrap();
        fs::create_dir_all(base.join("CPP")).unwrap();
        fs::write(base.join("C").join("foo.c"), "").unwrap();
        fs::write(base.join("CPP").join("foo.cpp"), "").unwrap();
        match resolve_target_in(&base, "foo") {
            ResolvedTarget::Ambiguous(paths) => assert_eq!(paths.len(), 2),
            other => panic!("expected Ambiguous, got {other:?}"),
        }
        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn reports_not_found() {
        let base = scratch_dir("notfound");
        match resolve_target_in(&base, "nope") {
            ResolvedTarget::NotFound => {}
            other => panic!("expected NotFound, got {other:?}"),
        }
        fs::remove_dir_all(&base).unwrap();
    }
}

fn read_libs() -> Vec<String> {
    fs::read_to_string("libs.txt")
        .map(|s| s.lines().filter(|l| !l.is_empty()).map(|l| l.to_string()).collect())
        .unwrap_or_default()
}

/// `cforge toolchain use <lang> <compiler>` (toolchain.rs) records a
/// compiler *family* (gcc/clang/mingw/msvc/apple-clang) in toolchain.txt;
/// `resolve_binary` turns that into the real binary name (g++ vs gcc,
/// clang++ vs clang, and the real Homebrew gcc-NN on macOS instead of
/// Apple clang's `gcc` alias). Absent an override, cmake picks its own
/// default (PKG_CONFIG_PATH env var, set by the caller, covers macOS).
fn compiler_override(lang: &str) -> Option<String> {
    crate::toolchain::compiler_for(lang).map(|family| crate::toolchain::resolve_binary(lang, &family))
}

fn configure_extra_args() -> Vec<String> {
    let mut args = Vec::new();
    let libs = read_libs();

    let c = compiler_override("c");
    let cxx = compiler_override("cpp");

    match os() {
        Os::Windows => {
            args.push("-G".to_string());
            args.push("Ninja".to_string());
            args.push(format!("-DCMAKE_C_COMPILER={}", c.unwrap_or_else(|| platform::c_compiler().to_string())));
            args.push(format!("-DCMAKE_CXX_COMPILER={}", cxx.unwrap_or_else(|| platform::cxx_compiler().to_string())));
            if !libs.is_empty() {
                args.push(format!("-DCMAKE_TOOLCHAIN_FILE={}", vcpkg_toolchain_file().display()));
            }
        }
        _ => {
            if let Some(c) = c {
                args.push(format!("-DCMAKE_C_COMPILER={c}"));
            }
            if let Some(cxx) = cxx {
                args.push(format!("-DCMAKE_CXX_COMPILER={cxx}"));
            }
        }
    }
    args
}

/// CMakeLists.txt resolves every `libs.txt` entry via `pkg_check_modules`
/// (pkg-config), on every platform including Windows — even though
/// `add_library` installs Windows libraries through vcpkg, not a
/// pkg-config-aware manager. vcpkg *does* emit `.pc` files for many (not
/// all) ports, so pointing PKG_CONFIG_PATH at vcpkg's install tree makes
/// those resolve the same way Homebrew's do on macOS. A port that doesn't
/// ship a `.pc` file has no generic fix here — vcpkg's CMake integration
/// for those needs a per-package `find_package()`/target name, which a
/// name-only `add <lib>` wrapper can't guess.
fn extra_pkg_config_path() -> Option<String> {
    if read_libs().is_empty() {
        return None;
    }
    let dirs: Vec<String> = match os() {
        Os::Macos if command_exists("brew") => {
            let out = Command::new("brew").arg("--prefix").output().ok()?;
            let prefix = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let opt = Path::new(&prefix).join("opt");
            fs::read_dir(&opt)
                .map(|entries| {
                    entries
                        .flatten()
                        .filter_map(|e| {
                            let pkgconfig = e.path().join("lib").join("pkgconfig");
                            pkgconfig.is_dir().then(|| pkgconfig.to_string_lossy().to_string())
                        })
                        .collect()
                })
                .unwrap_or_default()
        }
        Os::Windows => {
            let pkgconfig = vcpkg_root().join("installed").join(vcpkg_triplet()).join("lib").join("pkgconfig");
            if pkgconfig.is_dir() { vec![pkgconfig.to_string_lossy().to_string()] } else { Vec::new() }
        }
        _ => Vec::new(),
    };
    if dirs.is_empty() {
        return None;
    }
    let sep = if os() == Os::Windows { ";" } else { ":" };
    let existing = std::env::var("PKG_CONFIG_PATH").unwrap_or_default();
    Some(format!("{}{sep}{existing}", dirs.join(sep)))
}

/// pkg-config itself isn't part of a bare Windows/winget install; vcpkg
/// ships `pkgconf` (a pkg-config-compatible implementation) as an
/// installable port, so bootstrap it there when libraries are in use.
fn ensure_pkg_config_windows() {
    if command_exists("pkg-config") || command_exists("pkgconf") {
        return;
    }
    println!("pkg-config not found; installing pkgconf via vcpkg...");
    Command::new(vcpkg_exe()).args(["install", "pkgconf"]).status().ok();
}

/// Currently enabled languages: an empty/absent `langs.txt` means "all
/// platform-supported languages", matching the convention `project.rs`
/// already uses for `lang remove`/`lang list`/`info`.
fn enabled_langs() -> Vec<String> {
    let current = crate::project::read_langs_file();
    if current.is_empty() {
        crate::project::all_langs().iter().map(|s| s.to_string()).collect()
    } else {
        current
    }
}

fn discover_all_targets() -> Vec<TargetFile> {
    let mut names = std::collections::BTreeSet::new();
    let enabled = enabled_langs();
    let cfg = Config::load();

    // Build list of (dir, ext, lang) tuples using config paths
    let all_dirs: Vec<(&str, &str, &str)> = if crate::project::objc_capable_platform() {
        vec![
            (cfg.src_dir("c"), "c", "c"),
            (cfg.src_dir("cpp"), "cpp", "cpp"),
            (cfg.src_dir("obj_c"), "m", "obj_c"),
            (cfg.src_dir("obj_cpp"), "mm", "obj_cpp"),
        ]
    } else {
        vec![
            (cfg.src_dir("c"), "c", "c"),
            (cfg.src_dir("cpp"), "cpp", "cpp"),
        ]
    };

    let dirs: Vec<(&str, &str)> =
        all_dirs.iter().filter(|(_, _, lang)| enabled.iter().any(|l| l == lang)).map(|(d, e, _)| (*d, *e)).collect();
    for (dir, ext) in dirs {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                if entry.path().extension().and_then(|e| e.to_str()) == Some(ext) {
                    if let Some(stem) = entry.path().file_stem().and_then(|s| s.to_str()) {
                        // *_ffi.cpp (from `cforge new --ffi rust`) builds as a
                        // SHARED library in CMakeLists.txt, not an executable —
                        // see cmake.rs's FFI_CPP_FILES handling. It has no
                        // main() and no "<name>_ffi_cpp" executable target
                        // exists, so treating it as a normal build target here
                        // would just fail with "no rule to make target".
                        if stem.ends_with("_ffi") {
                            continue;
                        }
                        names.insert(stem.to_string());
                    }
                }
            }
        }
    }
    names
        .into_iter()
        .filter_map(|name| match resolve_target(&name) {
            ResolvedTarget::Found(t) => Some(t),
            ResolvedTarget::NotFound => None,
            // Two files sharing a base name across languages (e.g.
            // C/creditCard.c and CPP/creditCard.cpp) can't both become a
            // plain `creditCard` build target — silently dropping both, as
            // this used to do, meant a `cforge build` with no arguments
            // built neither and said nothing. Warn and skip instead; the
            // extension-qualified form (`cforge build creditCard.c`)
            // builds either one explicitly.
            ResolvedTarget::Ambiguous(paths) => {
                let list = paths.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(", ");
                eprintln!("warning: '{name}' is ambiguous ({list}) — skipped. Build it explicitly, e.g. 'cforge build {name}.c'.");
                None
            }
        })
        .collect()
}

/// Maps a `TargetFile::suffix` (as produced by `resolve_target_in`, e.g.
/// "objc") to the language name used in langs.txt / `cforge lang` (e.g.
/// "obj_c") — the two are spelled differently.
fn suffix_to_lang(suffix: &str) -> &str {
    match suffix {
        "objc" => "obj_c",
        "objcpp" => "obj_cpp",
        other => other,
    }
}

fn resolve_or_die(name: &str) -> TargetFile {
    match resolve_target(name) {
        ResolvedTarget::Found(t) => {
            let lang = suffix_to_lang(t.suffix);
            if !enabled_langs().iter().any(|l| l == lang) {
                eprintln!("Error: language '{lang}' is not enabled for this project — run 'cforge lang add {lang}' first.");
                std::process::exit(1);
            }
            t
        }
        ResolvedTarget::NotFound => {
            eprintln!("Error: no source file found for target '{name}' (looked for C/{name}.c, CPP/{name}.cpp{}).",
                if crate::project::objc_capable_platform() { format!(", Obj_C/{name}.m, Obj_CPP/{name}.mm") } else { String::new() });
            std::process::exit(1);
        }
        ResolvedTarget::Ambiguous(paths) => {
            let list = paths.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(", ");
            crate::usage_error(&format!(
                "'{name}' is ambiguous: found {list} — disambiguate with the extension, e.g. 'cforge build {name}.c' or 'cforge build {name}.cpp'"
            ));
        }
    }
}

fn configure(build_dir: &Path) {
    // Installs a compiler if the machine has none, and picks between them
    // if it has several. No-op once this project has one pinned.
    crate::toolchain::ensure_compiler_selected();
    ensure_cmake();
    if os() == Os::Windows {
        ensure_ninja();
        if !read_libs().is_empty() {
            ensure_vcpkg();
            ensure_pkg_config_windows();
        }
    }
    platform::create_dir_all(build_dir).unwrap_or_else(|e| {
        eprintln!("Error: could not create {}: {e}", build_dir.display());
        std::process::exit(1);
    });

    let flags = crate::flags::get();
    let mut args = configure_extra_args();
    if let Some(profile) = flags.profile {
        args.push(format!("-DCMAKE_BUILD_TYPE={}", profile.cmake_value()));
    }

    let mut configure_cmd = Command::new("cmake");
    configure_cmd.arg("..").args(&args).current_dir(build_dir);
    if let Some(pkg_config_path) = extra_pkg_config_path() {
        configure_cmd.env("PKG_CONFIG_PATH", pkg_config_path);
    }
    if flags.verbose >= 2 || flags.dry_run {
        println!("+ cmake .. {} (in {})", args.join(" "), build_dir.display());
    }
    if flags.dry_run {
        return;
    }
    let status = configure_cmd
        .stdout(std::process::Stdio::null())
        .status()
        .unwrap_or_else(|e| {
            eprintln!("Error: failed to run cmake: {e}");
            std::process::exit(1);
        });
    if !status.success() {
        eprintln!("Error: cmake configure failed.");
        std::process::exit(1);
    }
}

fn build_target(build_dir: &Path, target_file: &TargetFile) -> String {
    let name = target_file.path.file_stem().unwrap().to_string_lossy().to_string();
    let target = format!("{name}_{}", target_file.suffix);

    let flags = crate::flags::get();
    let mut args = vec!["--build".to_string(), ".".to_string(), "--target".to_string(), target.clone()];
    if let Some(jobs) = flags.jobs {
        args.push("-j".to_string());
        args.push(jobs.to_string());
    }
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    platform::run_or_die_in(build_dir, "cmake", &arg_refs, &format!("build failed for target '{target}'"));
    target
}

/// Finds `<cpp_src>/*_ffi.cpp` files (from `cforge new --ffi rust`) and
/// returns the shared-library target name CMakeLists.txt gives each one
/// (the "_ffi" suffix stripped — see cmake.rs's FFI_CPP_FILES handling).
/// These aren't `TargetFile`s: they have no `main()`, so they're excluded
/// from `discover_all_targets`/`resolve_target_in` and built separately.
fn discover_ffi_lib_targets() -> Vec<String> {
    let cfg = Config::load();
    let dir = cfg.src_dir("cpp");
    let Ok(entries) = fs::read_dir(dir) else { return Vec::new() };
    entries
        .flatten()
        .filter(|e| e.path().extension().and_then(|e| e.to_str()) == Some("cpp"))
        .filter_map(|e| e.path().file_stem().and_then(|s| s.to_str()).map(|s| s.to_string()))
        .filter_map(|stem| stem.strip_suffix("_ffi").map(|s| s.to_string()))
        .collect()
}

fn build_named_target(build_dir: &Path, target: &str) {
    let flags = crate::flags::get();
    let mut args = vec!["--build".to_string(), ".".to_string(), "--target".to_string(), target.to_string()];
    if let Some(jobs) = flags.jobs {
        args.push("-j".to_string());
        args.push(jobs.to_string());
    }
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    platform::run_or_die_in(build_dir, "cmake", &arg_refs, &format!("build failed for target '{target}'"));
}

/// Builds the given targets (all targets found if `targets` is empty).
/// Returns the resolved TargetFile for each one that was built, in the
/// order they were built, so callers like `run()` can reuse the
/// resolution instead of re-scanning the filesystem.
pub fn build(targets: &[String]) -> Vec<TargetFile> {
    let build_dir = Path::new("build");
    configure(build_dir);

    let resolved: Vec<TargetFile> = if targets.is_empty() {
        discover_all_targets()
    } else {
        targets.iter().map(|name| resolve_or_die(name)).collect()
    };

    for t in &resolved {
        build_target(build_dir, t);
        platform::status(&format!("Built {}", t.path.display()));
    }

    // Only auto-build FFI shared libraries on the no-args "build everything"
    // path — an explicit `cforge build <name>` should build exactly what
    // was asked for, not silently pull in the FFI library too.
    if targets.is_empty() {
        for lib_target in discover_ffi_lib_targets() {
            build_named_target(build_dir, &lib_target);
            platform::status(&format!("Built lib{lib_target} (FFI)"));
        }
    }

    resolved
}

/// Builds a single target then executes it, forwarding `program_args`
/// (everything the caller passed after `--`) and the child's exit code.
pub fn run(target: &str, program_args: &[String]) -> ! {
    let built = build(std::slice::from_ref(&target.to_string()));
    let target_file = built.into_iter().next().unwrap_or_else(|| {
        eprintln!("Error: nothing was built for target '{target}'.");
        std::process::exit(1);
    });

    let name = target_file.path.file_stem().unwrap().to_string_lossy().to_string();
    let exe_name = format!("{name}_{}", target_file.suffix);
    let build_dir = Path::new("build");
    let exe = build_dir.join(&exe_name);

    if crate::flags::get().dry_run {
        println!("+ {} {}", exe.display(), program_args.join(" "));
        std::process::exit(0);
    }

    let exe_win = build_dir.join(format!("{exe_name}.exe"));
    let exe_path = if exe.is_file() {
        exe
    } else if exe_win.is_file() {
        exe_win
    } else {
        eprintln!("Error: could not find executable 'build/{exe_name}' after building.");
        std::process::exit(1);
    };

    platform::status(&format!("Running {}...", exe_path.file_name().unwrap().to_string_lossy()));
    let status = Command::new(&exe_path).args(program_args).status();
    match status {
        Ok(s) => std::process::exit(s.code().unwrap_or(1)),
        Err(e) => {
            eprintln!("Error: could not run {}: {e}", exe_path.display());
            std::process::exit(1);
        }
    }
}

pub fn clean(all: bool) {
    let build_dir = Path::new("build");
    if !build_dir.exists() {
        platform::status("Nothing to clean (no build/ directory).");
        return;
    }
    if all {
        if crate::flags::get().dry_run {
            println!("+ rm -rf {}", build_dir.display());
            return;
        }
        fs::remove_dir_all(build_dir).unwrap_or_else(|e| {
            eprintln!("Error: could not remove {}: {e}", build_dir.display());
            std::process::exit(1);
        });
        platform::status("Removed build/ (including the CMake cache).");
    } else {
        platform::run_or_die_in(build_dir, "cmake", &["--build", ".", "--target", "clean"], "clean failed");
        platform::status("Cleaned build outputs (CMake cache kept).");
    }
}

/// Named `run_tests` rather than `test` to avoid any confusion with
/// `#[cfg(test)]` in this same file.
pub fn run_tests(filter: Option<&str>) {
    let build_dir = Path::new("build");
    if !build_dir.join("CMakeCache.txt").exists() {
        eprintln!("Error: no configured build in build/ — run 'cforge build' first.");
        std::process::exit(1);
    }
    let flags = crate::flags::get();
    let mut args = vec!["--output-on-failure".to_string()];
    if let Some(f) = filter {
        args.push("-R".to_string());
        args.push(f.to_string());
    }
    if let Some(jobs) = flags.jobs {
        args.push("-j".to_string());
        args.push(jobs.to_string());
    }
    if let Some(profile) = flags.profile {
        args.push("--build-config".to_string());
        args.push(profile.cmake_value().to_string());
    }
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    if !platform::run_in(build_dir, "ctest", &arg_refs) {
        std::process::exit(1);
    }
}

pub fn add_library(lib: &str) {
    if lib.is_empty() {
        crate::usage();
    }

    let libs = read_libs();
    if libs.iter().any(|l| l == lib) {
        println!("'{lib}' is already added.");
        return;
    }

    // Try cross-platform installation
    let Some(resolved_pkg) = crate::pkgmgr::install_library(lib) else {
        eprintln!("Error: failed to install '{lib}'.");
        eprintln!("Tried common package names for this platform.");
        eprintln!("If the library is installed manually, run: cforge add {lib}");
        std::process::exit(1);
    };

    crate::pkgmgr::add_to_libs(lib);

    let mut lock = crate::deps_lock::DepsLock::load();
    lock.record(lib, &resolved_pkg, crate::pkgmgr::platform_key());
    lock.save();

    platform::status(&format!("Added '{lib}' (resolved to package '{resolved_pkg}'). It will be linked into all targets on the next build."));
}

pub fn remove_library(lib: &str) {
    let libs = read_libs();
    if !libs.iter().any(|l| l == lib) {
        eprintln!("Error: '{lib}' is not currently added.");
        std::process::exit(1);
    }
    crate::pkgmgr::remove_from_libs(lib);

    let mut lock = crate::deps_lock::DepsLock::load();
    lock.remove(lib);
    lock.save();

    platform::status(&format!("Removed '{lib}'."));
}

pub fn list_libs() {
    let libs = read_libs();
    if libs.is_empty() {
        println!("No libraries added yet.");
    } else {
        for lib in libs {
            println!("{lib}");
        }
    }
}

/// `cforge deps show`: prints what's tracked in libs.txt alongside the
/// package name each one resolved to on this platform, per deps.lock —
/// makes it obvious at a glance whether a library still needs pinning.
pub fn deps_show() {
    let libs = read_libs();
    if libs.is_empty() {
        println!("No libraries added yet.");
        return;
    }
    let lock = crate::deps_lock::DepsLock::load();
    let platform = crate::pkgmgr::platform_key();
    for lib in libs {
        match lock.find(&lib, platform) {
            Some(entry) => println!("{lib} -> {} ({platform})", entry.package),
            None => println!("{lib} -> (not locked for {platform}; run 'cforge deps sync')"),
        }
    }
}

/// `cforge deps sync`: reinstalls every library in libs.txt, preferring the
/// exact package name pinned in deps.lock for this platform over
/// re-guessing from pkgmgr's candidate list. This is what a teammate or CI
/// runs on a fresh checkout instead of repeating `cforge add` for everything.
pub fn deps_sync() {
    let libs = read_libs();
    if libs.is_empty() {
        println!("No libraries to sync.");
        return;
    }
    let platform = crate::pkgmgr::platform_key();
    let mut lock = crate::deps_lock::DepsLock::load();
    let mut failed = Vec::new();

    for lib in &libs {
        if let Some(entry) = lock.find(lib, platform) {
            let pkg = entry.package.clone();
            platform::status(&format!("Installing '{lib}' (locked package: {pkg})..."));
            install_pkg(&pkg);
            if crate::pkgmgr::verify_pkg_config(lib) {
                continue;
            }
            platform::status(&format!("Locked package '{pkg}' for '{lib}' didn't verify; re-resolving..."));
        }
        match crate::pkgmgr::install_library(lib) {
            Some(resolved) => lock.record(lib, &resolved, platform),
            None => failed.push(lib.clone()),
        }
    }

    lock.save();

    if failed.is_empty() {
        platform::status("All dependencies synced.");
    } else {
        eprintln!("Error: failed to install: {}", failed.join(", "));
        std::process::exit(1);
    }
}

pub fn search(query: &str) {
    if query.is_empty() {
        crate::usage();
    }

    if os() == Os::Windows {
        ensure_vcpkg();
        println!("Searching vcpkg for '{query}'...");
        Command::new(vcpkg_exe()).args(["search", query]).status().ok();
        return;
    }

    let mgr = ensure_pkg_manager();
    println!("Searching for '{query}'...");
    match mgr {
        PkgManager::Brew => { Command::new("brew").args(["search", query]).status().ok(); }
        PkgManager::Apt => { Command::new("apt-cache").args(["search", query]).status().ok(); }
        PkgManager::Dnf => { Command::new("dnf").args(["search", query]).status().ok(); }
        PkgManager::Yum => { Command::new("yum").args(["search", query]).status().ok(); }
        PkgManager::Pacman => { Command::new("pacman").args(["-Ss", query]).status().ok(); }
        PkgManager::Zypper => { Command::new("zypper").args(["search", query]).status().ok(); }
        PkgManager::Apk => { Command::new("apk").args(["search", query]).status().ok(); }
        PkgManager::Winget | PkgManager::None => {}
    }
}

#[cfg(test)]
mod libs_tests {
    use super::*;
    use std::fs;

    fn in_scratch_dir<T>(label: &str, f: impl FnOnce() -> T) -> T {
        let dir = std::env::temp_dir().join(format!("cforge_test_libs_{label}_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();
        let result = f();
        std::env::set_current_dir(original).unwrap();
        fs::remove_dir_all(&dir).ok();
        result
    }

    #[test]
    fn remove_deletes_only_matching_line() {
        in_scratch_dir("remove", || {
            fs::write("libs.txt", "openssl\nsqlite3\n").unwrap();
            remove_library("openssl");
            let remaining = fs::read_to_string("libs.txt").unwrap();
            assert_eq!(remaining, "sqlite3\n");
        });
    }
}
