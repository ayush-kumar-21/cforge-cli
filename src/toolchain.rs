//! `cforge toolchain list|install|use|default|remove`.
//!
//! `toolchain.txt` holds `<lang>=<compiler>` lines. Unlike the bash version
//! (which only recorded the mapping), `use` here actually reconfigures the
//! build: `build::configure` reads this file and passes
//! `-DCMAKE_C_COMPILER=<compiler>` / `-DCMAKE_CXX_COMPILER=<compiler>` to
//! cmake for c/cpp entries.
use crate::color;
use crate::platform::{self, command_exists, ensure_pkg_manager, install_pkg, os, run_or_die, Os, PkgManager};
use crate::project::validate_lang;
use std::fs;
use std::io::{IsTerminal, Write};
use std::path::Path;

const TOOLCHAIN_FILE: &str = "toolchain.txt";

pub fn read_toolchain_file() -> Vec<(String, String)> {
    fs::read_to_string(TOOLCHAIN_FILE)
        .map(|s| s.lines().filter_map(|l| l.split_once('=')).map(|(k, v)| (k.to_string(), v.to_string())).collect())
        .unwrap_or_default()
}

fn write_toolchain_file(entries: &[(String, String)]) {
    let content: String = entries.iter().map(|(k, v)| format!("{k}={v}\n")).collect();
    platform::write_file(Path::new(TOOLCHAIN_FILE), &content);
}

/// Looks up the compiler override `build::configure` should pass to cmake
/// for a given cforge language ("c"/"cpp"), or `None` for platform default.
pub fn compiler_for(lang: &str) -> Option<String> {
    read_toolchain_file().into_iter().find(|(k, _)| k == lang).map(|(_, v)| v)
}

pub fn list() {
    let found = platform::detect_compilers();
    if found.is_empty() {
        println!("No C/C++ compiler found (run 'cforge build' and cforge will install one).");
    } else {
        println!("Detected compilers:");
        for c in &found {
            println!("  {} ({} / {})", c.version, c.c, c.cxx);
        }
    }
    if os() == Os::Macos {
        let xcode = platform::run_capture("xcode-select", &["-p"]).unwrap_or_else(|| "not found".to_string());
        println!("xcode-select: {xcode}");
    }
    let entries = read_toolchain_file();
    if !entries.is_empty() {
        println!();
        println!("project overrides ({TOOLCHAIN_FILE}):");
        for (k, v) in entries {
            println!("  {k}={v}");
        }
    }
}

/// Ensures this project has a compiler chosen, installing one first if the
/// machine has none.
///
/// The three cases a fresh user hits:
///
/// * No compiler at all: `ensure_compiler` installs one, nothing to pick.
/// * Exactly one: use it, and don't write toolchain.txt at all — an
///   unpinned project keeps following the system default, which is the
///   friendlier behaviour if they later switch compilers.
/// * More than one: ask, and pin the answer to toolchain.txt so it's
///   asked exactly once per project.
///
/// Idempotent: returns immediately once toolchain.txt names a compiler, so
/// this is safe to call on every build.
pub fn ensure_compiler_selected() {
    // Unconditionally first: a project can be pinned to a compiler on a
    // machine that has none (fresh clone of someone else's repo), and that
    // still needs the install. Cheap when one is already present.
    platform::ensure_compiler();
    if compiler_for("c").is_some() || compiler_for("cpp").is_some() {
        return;
    }

    let found = platform::detect_compilers();
    if found.len() < 2 {
        return;
    }

    // Never block for input where nothing can answer: CI, a piped stdin, or
    // --quiet (which is a request for no chatter, not a hidden prompt).
    // --dry-run additionally must not write toolchain.txt.
    let flags = crate::flags::get();
    let interactive = std::io::stdin().is_terminal() && !flags.quiet && !flags.dry_run;
    if !interactive {
        platform::status(&format!(
            "Multiple compilers detected; using {} (run 'cforge toolchain use c <compiler>' to change).",
            found[0].version
        ));
        return;
    }

    println!("Multiple C/C++ compilers found on this system:");
    for (i, c) in found.iter().enumerate() {
        println!("  {}) {}", i + 1, c.version);
    }
    let choice = prompt_index(found.len());
    let picked = &found[choice];

    write_toolchain_file(&[("c".to_string(), picked.c.clone()), ("cpp".to_string(), picked.cxx.clone())]);
    platform::status(&format!(
        "Using {} for this project ({} / {}). Change it any time with 'cforge toolchain use'.",
        picked.version, picked.c, picked.cxx
    ));
}

/// Reads a 1-based menu choice, re-asking until it's in range. EOF (^D, or
/// stdin closing mid-prompt) falls back to the first entry rather than
/// looping forever on a stream that will never produce input again.
fn prompt_index(len: usize) -> usize {
    loop {
        print!("Select a compiler for this project [1-{len}] (default 1): ");
        let _ = std::io::stdout().flush();

        let mut line = String::new();
        match std::io::stdin().read_line(&mut line) {
            Ok(0) | Err(_) => return 0,
            Ok(_) => {}
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return 0;
        }
        match trimmed.parse::<usize>() {
            Ok(n) if n >= 1 && n <= len => return n - 1,
            _ => eprintln!("Please enter a number between 1 and {len}."),
        }
    }
}

fn install_objc_macos(wait: bool) {
    let has_clt = platform::run_capture("xcode-select", &["-p"]).is_some() && command_exists("clang");
    if has_clt {
        platform::status("apple-clang already installed.");
        return;
    }

    platform::status("Command Line Tools not found; launching the installer...");
    run_or_die("xcode-select", &["--install"], "failed to launch the Command Line Tools installer");

    if !wait {
        platform::status("A GUI installer was launched. Complete it, then re-run this command.");
        std::process::exit(4);
    }
    if crate::flags::get().dry_run {
        return;
    }
    platform::wait_for_xcode_clt();
}

fn install_native(lang: &str, compiler: Option<&str>, version: Option<&str>) {
    let Some(compiler) = compiler else {
        crate::platform::ensure_compiler();
        platform::status(&format!("default compiler for {lang} already available."));
        return;
    };

    let pkg = match version {
        Some(v) => format!("{compiler}@{v}"),
        None => compiler.to_string(),
    };
    match compiler {
        "gcc" | "clang" => install_pkg(&pkg),
        "mingw" => match ensure_pkg_manager() {
            // MSYS2's pacman and winget don't share a package namespace —
            // "mingw-w64-x86_64-toolchain" is a pacman-only package name.
            // The winget id below is best-effort: I can't verify it exists
            // from this sandbox (no Windows machine, no network access to
            // winget's catalog) — if it's wrong, `install_pkg` will fail
            // loudly rather than silently, but treat it as unverified.
            PkgManager::Pacman => install_pkg("mingw-w64-x86_64-toolchain"),
            PkgManager::Winget => install_pkg("BrechtSanders.WinLibs.POSIX.UCRT"),
            _ => {
                eprintln!("Error: installing mingw needs MSYS2 (https://www.msys2.org) with pacman, or winget.");
                std::process::exit(1);
            }
        },
        "msvc" | "apple-clang" => {
            platform::status(&format!("{compiler} ships with the platform toolchain; nothing to install."))
        }
        other => crate::usage_error(&format!("unknown --compiler '{other}'")),
    }

    let mut entries: Vec<(String, String)> = read_toolchain_file().into_iter().filter(|(k, _)| k != lang).collect();
    entries.push((lang.to_string(), compiler.to_string()));
    write_toolchain_file(&entries);
    platform::status(&format!("installed {compiler} for {lang}"));
}

/// Resolves a compiler *family* (as typed by the user, or read back from
/// toolchain.txt) to the actual binary cmake should invoke for `lang`.
/// A family name alone is ambiguous in two ways: it doesn't say which
/// language binary you want (gcc's C++ counterpart is g++, not gcc), and
/// on macOS "gcc"/"g++" are themselves aliases for Apple clang unless
/// resolved to a real versioned Homebrew binary.
pub fn resolve_binary(lang: &str, family: &str) -> String {
    match (lang, family) {
        ("c", "gcc") => platform::resolve_real_gnu_compiler("gcc").unwrap_or_else(|| "gcc".to_string()),
        ("cpp", "gcc") => platform::resolve_real_gnu_compiler("g++").unwrap_or_else(|| "g++".to_string()),
        ("cpp", "clang") => "clang++".to_string(),
        ("c", "mingw") => "gcc".to_string(),
        ("cpp", "mingw") => "g++".to_string(),
        (_, other) => other.to_string(),
    }
}

pub fn install(lang: &str, compiler: Option<&str>, version: Option<&str>, wait: bool) {
    validate_lang(lang);
    match lang {
        // validate_lang already exited if this is not macOS: obj_c/obj_cpp
        // are Unsupported everywhere else, so there is no second branch.
        "obj_c" | "obj_cpp" => {
            if compiler.is_some() {
                eprintln!("Error: --compiler is not valid for {lang}");
                eprintln!("  Objective-C requires apple-clang; it is the only toolchain that");
                eprintln!("  links Foundation and AppKit. Homebrew gcc provides gobjc but");
                eprintln!("  cannot link Apple frameworks.");
                std::process::exit(2);
            }
            install_objc_macos(wait);
        }
        "c" | "cpp" => install_native(lang, compiler, version),
        _ => unreachable!("validate_lang already rejected anything else"),
    }
}

pub fn use_compiler(lang: &str, compiler: &str) {
    validate_lang(lang);
    let mut entries: Vec<(String, String)> = read_toolchain_file().into_iter().filter(|(k, _)| k != lang).collect();
    entries.push((lang.to_string(), compiler.to_string()));
    write_toolchain_file(&entries);
    platform::status(&format!("toolchain for {lang} set to {compiler} (applied on the next 'cforge build')"));
}

pub fn default_toolchain() {
    if Path::new(TOOLCHAIN_FILE).exists() {
        if crate::flags::get().dry_run {
            println!("+ rm {TOOLCHAIN_FILE}");
        } else {
            fs::remove_file(TOOLCHAIN_FILE).ok();
        }
    }
    platform::status("toolchain mappings reset to system defaults.");
}

/// The bash version's `toolchain remove` was a bare package-manager
/// uninstall with no check that the compiler being removed isn't the last
/// one configured for a language — this refuses that case instead of
/// silently leaving a project unbuildable.
pub fn remove(compiler: &str) {
    let entries = read_toolchain_file();
    let still_needed: Vec<&str> = entries.iter().filter(|(_, v)| v == compiler).map(|(k, _)| k.as_str()).collect();
    if !still_needed.is_empty() {
        eprintln!(
            "Error: '{compiler}' is still configured for: {} (run 'cforge toolchain use <lang> <other-compiler>' first, or 'cforge toolchain default' to reset).",
            still_needed.join(", ")
        );
        std::process::exit(1);
    }

    match ensure_pkg_manager() {
        PkgManager::Brew => run_or_die("brew", &["uninstall", compiler], "failed to uninstall"),
        PkgManager::Apt => run_or_die("sudo", &["apt-get", "remove", "-y", compiler], "failed to uninstall"),
        PkgManager::Dnf => run_or_die("sudo", &["dnf", "remove", "-y", compiler], "failed to uninstall"),
        PkgManager::Yum => run_or_die("sudo", &["yum", "remove", "-y", compiler], "failed to uninstall"),
        PkgManager::Zypper => run_or_die("sudo", &["zypper", "remove", "-y", compiler], "failed to uninstall"),
        PkgManager::Apk => run_or_die("sudo", &["apk", "del", compiler], "failed to uninstall"),
        PkgManager::Pacman => run_or_die("sudo", &["pacman", "-R", "--noconfirm", compiler], "failed to uninstall"),
        PkgManager::Winget => run_or_die("winget", &["uninstall", "-e", "--id", compiler], "failed to uninstall"),
        PkgManager::None => unreachable!("ensure_pkg_manager exits before returning None"),
    }
    platform::status(&color::green(&format!("Removed '{compiler}'.")));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiler_for_reads_back_use_compiler() {
        crate::platform::in_scratch_dir("use", || {
            use_compiler("c", "gcc");
            assert_eq!(compiler_for("c"), Some("gcc".to_string()));
            assert_eq!(compiler_for("cpp"), None);
        });
    }

    #[test]
    fn use_compiler_replaces_existing_entry_for_same_lang() {
        crate::platform::in_scratch_dir("replace", || {
            use_compiler("c", "gcc");
            use_compiler("c", "clang");
            let entries = read_toolchain_file();
            assert_eq!(entries.iter().filter(|(k, _)| k == "c").count(), 1);
            assert_eq!(compiler_for("c"), Some("clang".to_string()));
        });
    }
}
