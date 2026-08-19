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
use std::path::Path;

const TOOLCHAIN_FILE: &str = "toolchain.txt";

pub fn read_toolchain_file() -> Vec<(String, String)> {
    fs::read_to_string(TOOLCHAIN_FILE)
        .map(|s| {
            s.lines()
                .filter_map(|l| l.split_once('='))
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect()
        })
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
    println!("cc:  {}", if command_exists("cc") { "found" } else { "not found" });
    println!("c++: {}", if command_exists("c++") { "found" } else { "not found" });
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

fn install_objc_linux(runtime: &str) {
    ensure_pkg_manager();
    match runtime {
        "gnustep" => {
            install_pkg("gobjc");
            install_pkg("gnustep-devel");
            install_pkg("gnustep-base");
            platform::status("installed: gobjc, gnustep-base");
        }
        "libobjc2" => {
            install_pkg("clang");
            install_pkg("libobjc2");
            install_pkg("gnustep-libobjc2");
            platform::status("installed: clang, libobjc2");
        }
        other => crate::usage_error(&format!("unknown --runtime '{other}' (expected: gnustep|libobjc2)")),
    }
    platform::write_file(Path::new(".cforge_objc_runtime"), &format!("{runtime}\n"));
    platform::status("note: Objective-C on Linux uses the GNUstep runtime.");
    platform::status("      Apple frameworks (Cocoa, AppKit, UIKit) are unavailable.");
    platform::status("      ARC requires --runtime libobjc2.");
    platform::status("      Run 'cforge info' to see this again.");
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
        "msvc" | "apple-clang" => platform::status(&format!("{compiler} ships with the platform toolchain; nothing to install.")),
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

pub fn install(lang: &str, compiler: Option<&str>, version: Option<&str>, runtime: &str, wait: bool) {
    validate_lang(lang);
    match lang {
        "obj_c" | "obj_cpp" => {
            if os() == Os::Macos {
                if compiler.is_some() {
                    eprintln!("Error: --compiler is not valid for {lang} on macOS");
                    eprintln!("  Objective-C on macOS requires apple-clang; it is the only toolchain");
                    eprintln!("  that links Foundation and AppKit. Homebrew gcc provides gobjc but");
                    eprintln!("  cannot link Apple frameworks.");
                    std::process::exit(2);
                }
                install_objc_macos(wait);
            } else {
                install_objc_linux(runtime);
            }
        }
        "c" | "cpp" => install_native(lang, compiler, version),
        "swift" => install_swift(),
        _ => unreachable!("validate_lang already rejected anything else"),
    }
}

/// macOS: swiftc ships with Xcode Command Line Tools, already covered by
/// `ensure_compiler`. Linux and Windows are automated against swift.org's
/// own documented install paths (verified live against
/// https://www.swift.org/install/linux/ and /windows/ — not guessed):
/// `swiftly` (the official installer) on Linux, `winget install
/// Swift.Toolchain` on Windows. Neither distro package managers (apt/dnf/
/// pacman) nor a plain winget-only flow are how swift.org actually
/// recommends installing Swift, so cforge doesn't pretend otherwise.
fn install_swift() {
    if command_exists("swiftc") {
        platform::status("swiftc already installed.");
        return;
    }
    match os() {
        Os::Macos => {
            crate::platform::ensure_compiler();
            if command_exists("swiftc") {
                platform::status("swiftc installed via Xcode Command Line Tools.");
            } else {
                eprintln!("Error: Command Line Tools installed but swiftc still not found; try a full Xcode install.");
                std::process::exit(1);
            }
        }
        Os::Linux => install_swift_linux(),
        Os::Windows => install_swift_windows(),
        Os::Unknown => {
            eprintln!("Error: unsupported OS; install Swift manually from https://www.swift.org/install/");
            std::process::exit(1);
        }
    }
}

/// Bootstraps `swiftly` exactly as swift.org's own linux install page
/// documents, then uses it to install a toolchain. `swiftly` (not apt/dnf/
/// pacman) is the officially documented path — there's no native distro
/// package for the compiler itself.
fn install_swift_linux() {
    platform::status("Installing Swift via swiftly (the official installer — see https://www.swift.org/install/linux/)...");
    let arch = platform::run_capture("uname", &["-m"]).unwrap_or_else(|| "x86_64".to_string());
    let tarball = format!("swiftly-{arch}.tar.gz");
    let bootstrap = format!(
        "curl -O https://download.swift.org/swiftly/linux/{tarball} && tar zxf {tarball} && ./swiftly init --quiet-shell-followup"
    );
    if crate::flags::get().dry_run {
        println!("+ sh -c '{bootstrap}'");
        println!("+ ~/.local/share/swiftly/bin/swiftly install latest");
        return;
    }
    run_or_die("sh", &["-c", &bootstrap], "failed to bootstrap swiftly (see https://www.swift.org/install/linux/)");
    let home = std::env::var("HOME").unwrap_or_default();
    let swiftly_bin = format!("{home}/.local/share/swiftly/bin/swiftly");
    run_or_die(&swiftly_bin, &["install", "latest"], "failed to install a Swift toolchain via swiftly");
    fs::remove_file(&tarball).ok();
    platform::status("Swift installed via swiftly. Open a new shell (or `source ~/.local/share/swiftly/env.sh`) so swiftc is on PATH.");
}

/// Installs the compiler via winget's `Swift.Toolchain` package, swift.org's
/// own documented Windows path. Deliberately does NOT also auto-install the
/// Visual Studio Build Tools / Windows SDK components swift.org's page lists
/// as a prerequisite: that's a multi-GB, system-altering install, not
/// something to trigger silently — it's surfaced as a command to run by hand.
fn install_swift_windows() {
    if !command_exists("winget") {
        eprintln!("Error: winget not found; install Swift manually from https://www.swift.org/install/windows/");
        std::process::exit(1);
    }
    platform::status("Installing Swift via winget (Swift.Toolchain — see https://www.swift.org/install/windows/)...");
    if crate::flags::get().dry_run {
        println!("+ winget install --id Swift.Toolchain -e --source winget");
        return;
    }
    run_or_die(
        "winget",
        &["install", "--id", "Swift.Toolchain", "-e", "--source", "winget"],
        "failed to install Swift.Toolchain via winget",
    );
    platform::status("Swift toolchain installed. Building also needs the MSVC/Windows SDK components; if 'cforge build' can't find them, run once:");
    platform::status(
        "  winget install --id Microsoft.VisualStudio.2022.Community --exact --force --custom \"--add Microsoft.VisualStudio.Component.Windows11SDK.22621 --add Microsoft.VisualStudio.Component.VC.Tools.x86.x64 --add Microsoft.VisualStudio.Component.VC.Tools.ARM64\" --source winget",
    );
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
    use std::fs;

    fn in_scratch_dir<T>(label: &str, f: impl FnOnce() -> T) -> T {
        let dir = std::env::temp_dir().join(format!("cforge_test_toolchain_{label}_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();
        let result = f();
        std::env::set_current_dir(original).unwrap();
        fs::remove_dir_all(&dir).ok();
        result
    }

    #[test]
    fn compiler_for_reads_back_use_compiler() {
        in_scratch_dir("use", || {
            use_compiler("c", "gcc");
            assert_eq!(compiler_for("c"), Some("gcc".to_string()));
            assert_eq!(compiler_for("cpp"), None);
        });
    }

    #[test]
    fn use_compiler_replaces_existing_entry_for_same_lang() {
        in_scratch_dir("replace", || {
            use_compiler("c", "gcc");
            use_compiler("c", "clang");
            let entries = read_toolchain_file();
            assert_eq!(entries.iter().filter(|(k, _)| k == "c").count(), 1);
            assert_eq!(compiler_for("c"), Some("clang".to_string()));
        });
    }
}
