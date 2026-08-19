use crate::cmake::generate_cmakelists;
use crate::config::Config;
use crate::platform::{self, ensure_compiler, os, Os};
use std::fs;
use std::path::Path;

const KNOWN_LANGS: &[&str] = &["c", "cpp", "obj_c", "obj_cpp"];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LangCapability {
    Supported,
    /// Allowed, but with rough edges the caller may want to warn about
    /// (Linux Obj-C/Obj-C++ via GNUstep: no Apple frameworks, ARC needs
    /// --runtime libobjc2).
    Caveats,
    Unsupported,
}

/// Mirrors the bash version's three-tier `lang_capability()`: c/cpp work
/// everywhere; obj_c/obj_cpp are fully supported on macOS, work with
/// caveats on Linux (GNUstep), and have no viable toolchain elsewhere
/// (notably Windows). Panics on an unknown language — callers are
/// expected to have already matched against `KNOWN_LANGS`.
pub fn lang_capability(lang: &str) -> LangCapability {
    match lang {
        "c" | "cpp" => LangCapability::Supported,
        "obj_c" | "obj_cpp" => match os() {
            Os::Macos => LangCapability::Supported,
            Os::Linux => LangCapability::Caveats,
            _ => LangCapability::Unsupported,
        },
        other => unreachable!("lang_capability called with unknown language '{other}'"),
    }
}

/// True if this platform has *any* viable obj_c/obj_cpp toolchain path
/// (macOS or Linux/GNUstep) — used to gate source-file discovery and
/// CMakeLists.txt generation, which need a yes/no rather than the full
/// three-tier capability.
pub fn objc_capable_platform() -> bool {
    matches!(os(), Os::Macos | Os::Linux)
}

/// Platform-supported languages, used as the default set when nothing is
/// pinned in langs.txt. Matches the bash version's
/// `default_langs_for_platform()`: every known language except on
/// Windows, where obj_c/obj_cpp have no viable toolchain at all.
pub fn all_langs() -> &'static [&'static str] {
    if objc_capable_platform() { &["c", "cpp", "obj_c", "obj_cpp"] } else { &["c", "cpp"] }
}

pub fn validate_lang(lang: &str) {
    if !KNOWN_LANGS.contains(&lang) {
        crate::usage_error(&format!("unknown language '{lang}' (expected: {})", KNOWN_LANGS.join("|")));
    }
    if lang_capability(lang) == LangCapability::Unsupported {
        eprintln!("Error: {lang} is not supported on this platform.");
        eprintln!("  Objective-C and Objective-C++ are Apple platform languages;");
        eprintln!("  full support requires macOS. Linux has partial support via GNUstep.");
        eprintln!("  Supported here: c, cpp");
        std::process::exit(3);
    }
}

fn write_langs_file(langs: &[String]) {
    let content = langs.iter().map(|l| format!("{l}\n")).collect::<String>();
    platform::write_file(Path::new("langs.txt"), &content);
}

pub(crate) fn read_langs_file() -> Vec<String> {
    fs::read_to_string("langs.txt")
        .map(|s| s.lines().map(|l| l.to_string()).collect())
        .unwrap_or_default()
}

pub fn lang_add(langs: &[String]) {
    let mut current = read_langs_file();
    // No langs.txt yet: seed with platform defaults *before* applying the
    // requested add, so `lang add <lang>` on a fresh project extends the
    // default set instead of replacing it with just that one language (a
    // real regression this fixes — it previously only seeded defaults when
    // called with no arguments at all).
    if current.is_empty() {
        current = all_langs().iter().map(|s| s.to_string()).collect();
    }
    for l in langs {
        validate_lang(l);
        if !current.contains(l) {
            current.push(l.clone());
        }
    }
    current.sort();
    current.dedup();
    write_langs_file(&current);
    platform::status(&format!("Configured project for: {}", current.join(" ")));
}

pub fn lang_remove(langs: &[String]) {
    if langs.is_empty() {
        crate::usage_error("lang remove requires at least one language");
    }
    let mut current = read_langs_file();
    if current.is_empty() {
        current = all_langs().iter().map(|s| s.to_string()).collect();
    }
    for lang in langs {
        validate_lang(lang);
        current.retain(|l| l != lang);
    }
    write_langs_file(&current);
    platform::status(&format!("Configured project for: {}", current.join(" ")));
}

pub fn lang_list() {
    let current = read_langs_file();
    if current.is_empty() {
        println!("(none pinned — building with all supported languages: {})", all_langs().join(" "));
    } else {
        for lang in current {
            println!("{lang}");
        }
    }
}

pub fn generate(name: &str, ext: &str) {
    if name.is_empty() {
        crate::usage_error("generate requires a name");
    }
    validate_lang(ext);
    let cfg = Config::load();
    let (dir, file_ext) = match ext {
        "c" => (cfg.src_dir("c"), "c"),
        "cpp" => (cfg.src_dir("cpp"), "cpp"),
        "obj_c" => (cfg.src_dir("obj_c"), "m"),
        "obj_cpp" => (cfg.src_dir("obj_cpp"), "mm"),
        _ => unreachable!("validate_lang already rejected anything else"),
    };
    let file = Path::new(dir).join(format!("{name}.{file_ext}"));
    if file.exists() {
        eprintln!("Error: {} already exists.", file.display());
        std::process::exit(1);
    }
    fs::create_dir_all(dir).unwrap();
    platform::write_file(&file, "");
    platform::status(&format!("Created {}", file.display()));
}

fn sanitize_project_name(raw: &str) -> String {
    let s: String = raw
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
        .collect();
    if s.is_empty() { "project".to_string() } else { s }
}

/// The project name as `init()` derives it: the current directory's
/// basename, sanitized. `new_project` passes whatever string the user
/// typed after `cforge new` (which may be a path like `/tmp/foo` or
/// `../foo`) — callers that need the *actual* project identifier (e.g.
/// FFI scaffolding naming its crate/symbols) should use this instead of
/// that raw argument, since directory name and CLI argument can differ.
pub fn current_project_name() -> String {
    let cwd = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
    let dir_name = cwd
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string());
    sanitize_project_name(&dir_name)
}

pub fn init() {
    let cmakelists = Path::new("CMakeLists.txt");
    if cmakelists.exists() {
        println!("CMakeLists.txt already exists; skipping.");
        return;
    }

    let project_name = current_project_name();

    platform::write_file(cmakelists, &generate_cmakelists(&project_name));
    crate::cmake::generate_cforge_config();

    let cfg = Config::load();
    // Only create directories for languages actually enabled for this
    // project (langs.txt if it's already been written, else the platform
    // default set) — not all 4 unconditionally. Otherwise every project
    // gets dead Obj_C/Obj_CPP folders on Windows, where they can never be
    // built.
    let enabled = read_langs_file();
    let enabled: Vec<&str> = if enabled.is_empty() { all_langs().to_vec() } else { enabled.iter().map(|s| s.as_str()).collect() };
    let src_dirs = enabled.iter().map(|lang| cfg.src_dir(lang));
    let dirs = src_dirs.chain(std::iter::once(cfg.paths.build.as_str()));
    for dir in dirs {
        fs::create_dir_all(dir).unwrap_or_else(|e| {
            eprintln!("Error: could not create {dir}: {e}");
            std::process::exit(1);
        });
    }
    platform::status("Initialized CMakeLists.txt and project directories.");
}

/// `langs` empty means "all platform-supported languages" (same default
/// as before). Standards are no longer set here — they always start at
/// whatever `init()`'s generated CMakeLists.txt defaults to (latest), and
/// `std set` is the only way to change them, matching the reference doc's
/// `new` command which takes no standards arguments at all.
pub fn new_project(name: &str, langs: &[String]) {
    if name.is_empty() {
        crate::usage_error("new requires a project name");
    }
    fs::create_dir_all(name).unwrap_or_else(|e| {
        eprintln!("Error: could not create directory '{name}': {e}");
        std::process::exit(1);
    });
    std::env::set_current_dir(name).unwrap_or_else(|e| {
        eprintln!("Error: could not enter directory '{name}': {e}");
        std::process::exit(1);
    });

    ensure_compiler();
    // An explicit --lang list is the exact set for this project, not an
    // addition to the platform defaults — `lang_add` (used by `lang add`,
    // where "add to what's there" is the right semantics) would otherwise
    // just merge it into the full default set. Empty means "no --lang
    // given", which still means "all platform-supported languages".
    let mut explicit: Vec<String> = if langs.is_empty() {
        all_langs().iter().map(|s| s.to_string()).collect()
    } else {
        for l in langs {
            validate_lang(l);
        }
        langs.to_vec()
    };
    explicit.sort();
    explicit.dedup();
    write_langs_file(&explicit);
    platform::status(&format!("Configured project for: {}", explicit.join(" ")));
    // init() reads langs.txt (written above) to decide which source
    // directories to create.
    init();
}

pub fn info() {
    let langs = read_langs_file();
    let langs_display = if langs.is_empty() {
        format!("{} (default: all supported)", all_langs().join(" "))
    } else {
        langs.join(" ")
    };
    println!("Languages: {langs_display}");

    let cmakelists = Path::new("CMakeLists.txt");
    if cmakelists.exists() {
        let std_row = |key: &str, var: &str| {
            let val = crate::cmake::get_cmake_var(cmakelists, var).unwrap_or_else(|| "(not set)".to_string());
            println!("  {key:<8} {val}");
        };
        println!("Standards:");
        std_row("c", "CMAKE_C_STANDARD");
        std_row("cxx", "CMAKE_CXX_STANDARD");
        if objc_capable_platform() {
            std_row("objc", "CMAKE_OBJC_STANDARD");
            std_row("objcpp", "CMAKE_OBJCXX_STANDARD");
        }
    } else {
        println!("Standards: (no CMakeLists.txt — run 'cforge init' first)");
    }

    let libs = fs::read_to_string("libs.txt")
        .map(|s| s.lines().filter(|l| !l.is_empty()).map(|l| l.to_string()).collect::<Vec<_>>())
        .unwrap_or_default();
    if libs.is_empty() {
        println!("Libraries: (none)");
    } else {
        println!("Libraries: {}", libs.join(" "));
    }
}

#[cfg(test)]
mod capability_tests {
    use super::*;

    /// Locks in the fix for the macOS-only regression: `all_langs()`'s
    /// obj_c/obj_cpp membership must always agree with
    /// `objc_capable_platform()`, on whatever host runs this test.
    #[test]
    fn all_langs_agrees_with_objc_capability() {
        assert_eq!(all_langs().contains(&"obj_c"), objc_capable_platform());
        assert_eq!(all_langs().contains(&"obj_cpp"), objc_capable_platform());
    }

    /// c/cpp are always Supported; obj_c/obj_cpp are never flatly
    /// Unsupported except where there's truly no toolchain path (Windows).
    #[test]
    fn capability_tiers_are_internally_consistent() {
        assert_eq!(lang_capability("c"), LangCapability::Supported);
        assert_eq!(lang_capability("cpp"), LangCapability::Supported);
        assert_eq!(lang_capability("obj_c") == LangCapability::Unsupported, !objc_capable_platform());
        assert_eq!(lang_capability("obj_cpp") == LangCapability::Unsupported, !objc_capable_platform());
    }
}
