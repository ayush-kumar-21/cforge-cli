use std::path::PathBuf;
use std::process::{Command, Stdio};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Os {
    Macos,
    Linux,
    Windows,
    Unknown,
}

pub fn os() -> Os {
    match std::env::consts::OS {
        "macos" => Os::Macos,
        "linux" => Os::Linux,
        "windows" => Os::Windows,
        _ => Os::Unknown,
    }
}

pub fn home_dir() -> PathBuf {
    if let Ok(h) = std::env::var("HOME") {
        return PathBuf::from(h);
    }
    if let Ok(h) = std::env::var("USERPROFILE") {
        return PathBuf::from(h);
    }
    PathBuf::from(".")
}

/// The C/C++ compiler binaries cforge builds against. macOS/Linux use the
/// portable `cc`/`c++` aliases. Native Windows has no such alias by
/// default; this respects an already-installed MinGW-w64/MSYS2 gcc/g++ if
/// one is on PATH (the toolchain a user coming from `pacman` expects), and
/// only falls back to LLVM's clang/clang++ (which `ensure_compiler`
/// installs via winget) when neither is present.
pub fn c_compiler() -> &'static str {
    if os() == Os::Windows {
        if command_exists("gcc") { "gcc" } else { "clang" }
    } else {
        "cc"
    }
}

pub fn cxx_compiler() -> &'static str {
    if os() == Os::Windows {
        if command_exists("g++") { "g++" } else { "clang++" }
    } else {
        "c++"
    }
}

/// Reimplementation of `which`/`command -v` using only std: scans PATH,
/// honoring PATHEXT on Windows.
pub fn command_path(name: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    let exts: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".EXE;.CMD;.BAT;.COM".to_string())
            .split(';')
            .map(|s| s.to_string())
            .collect()
    } else {
        vec![String::new()]
    };
    for dir in std::env::split_paths(&path_var) {
        for ext in &exts {
            let candidate = dir.join(format!("{name}{ext}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

pub fn command_exists(name: &str) -> bool {
    command_path(name).is_some()
}

/// Prints `msg` unless the user passed -q/--quiet.
pub fn status(msg: &str) {
    if !crate::flags::get().quiet {
        println!("{msg}");
    }
}

fn format_cmd(cmd: &str, args: &[&str]) -> String {
    let mut parts = vec![cmd.to_string()];
    parts.extend(args.iter().map(|s| s.to_string()));
    parts.join(" ")
}

pub fn run(cmd: &str, args: &[&str]) -> bool {
    let f = crate::flags::get();
    if f.verbose >= 2 || f.dry_run {
        println!("+ {}", format_cmd(cmd, args));
    }
    if f.dry_run {
        return true;
    }
    Command::new(cmd).args(args).status().map(|s| s.success()).unwrap_or(false)
}

pub fn run_or_die(cmd: &str, args: &[&str], context: &str) {
    if !run(cmd, args) {
        eprintln!("Error: {context} (`{cmd} {}` failed)", args.join(" "));
        std::process::exit(1);
    }
}

pub fn run_in(dir: &std::path::Path, cmd: &str, args: &[&str]) -> bool {
    let f = crate::flags::get();
    if f.verbose >= 2 || f.dry_run {
        println!("+ {} (in {})", format_cmd(cmd, args), dir.display());
    }
    if f.dry_run {
        return true;
    }
    Command::new(cmd).args(args).current_dir(dir).status().map(|s| s.success()).unwrap_or(false)
}

pub fn run_or_die_in(dir: &std::path::Path, cmd: &str, args: &[&str], context: &str) {
    if !run_in(dir, cmd, args) {
        eprintln!("Error: {context} (`{cmd} {}` failed)", args.join(" "));
        std::process::exit(1);
    }
}

/// Writes `content` to `path`, or under `--dry-run` prints what would have
/// been written instead. This is the single choke point for the
/// project-bookkeeping files (langs.txt, libs.txt, toolchain.txt,
/// CMakeLists.txt) so `--dry-run` covers local writes too, not just the
/// external commands `run`/`run_in` gate.
pub fn write_file(path: &std::path::Path, content: &str) {
    let f = crate::flags::get();
    if f.verbose >= 2 || f.dry_run {
        println!("+ write {}", path.display());
    }
    if f.dry_run {
        return;
    }
    std::fs::write(path, content).unwrap_or_else(|e| {
        eprintln!("Error: could not write {}: {e}", path.display());
        std::process::exit(1);
    });
}

/// Runs a command and returns its trimmed stdout on success. Ignores
/// dry-run (callers use this to *query* state, e.g. `cmake --version`,
/// never to mutate it), and does not print anything itself.
pub fn run_capture(cmd: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(cmd).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// macOS (and some Linux distros' `update-alternatives`) ship a bare
/// `gcc`/`g++` that's actually a thin wrapper around clang — Apple's clang
/// literally reports "Apple clang version ..." when invoked as `gcc
/// --version`. Real GNU GCC installed alongside it (e.g. `brew install
/// gcc`) lands as a versioned binary like `gcc-16`, never plain `gcc`.
/// Without this, `cforge toolchain use c gcc` would silently keep
/// building with clang while believing it's using GCC.
pub fn resolve_real_gnu_compiler(name: &str) -> Option<String> {
    if let Some(out) = run_capture(name, &["--version"]) {
        if !out.contains("clang") {
            return Some(name.to_string());
        }
    }
    for major in (9..=16).rev() {
        let versioned = format!("{name}-{major}");
        if command_exists(&versioned) {
            return Some(versioned);
        }
    }
    None
}

/// Probes whether `compiler -std=<flag> -x <lang> -E -` accepts an empty
/// input without erroring, same technique the original bash version used.
pub fn probe_std(compiler: &str, std_flag: &str, lang: &str) -> bool {
    let child = Command::new(compiler)
        .args([std_flag, "-x", lang, "-E", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(_) => return false,
    };
    drop(child.stdin.take());
    child.wait().map(|s| s.success()).unwrap_or(false)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PkgManager {
    Brew,
    Apt,
    Dnf,
    Yum,
    Pacman,
    Zypper,
    Apk,
    Winget,
    None,
}

pub fn detect_pkg_manager() -> PkgManager {
    match os() {
        Os::Macos => PkgManager::Brew,
        Os::Linux => {
            if command_exists("apt-get") {
                PkgManager::Apt
            } else if command_exists("dnf") {
                PkgManager::Dnf
            } else if command_exists("yum") {
                PkgManager::Yum
            } else if command_exists("pacman") {
                PkgManager::Pacman
            } else if command_exists("zypper") {
                PkgManager::Zypper
            } else if command_exists("apk") {
                PkgManager::Apk
            } else {
                PkgManager::None
            }
        }
        // MSYS2 (Git-Bash/MSYS2 shells put pacman on PATH) is preferred when
        // present: it's the strongest signal the user wants the native
        // MinGW-w64/gcc toolchain rather than winget+clang.
        Os::Windows => {
            if command_exists("pacman") {
                PkgManager::Pacman
            } else if command_exists("winget") {
                PkgManager::Winget
            } else {
                PkgManager::None
            }
        }
        Os::Unknown => PkgManager::None,
    }
}

fn bootstrap_homebrew() {
    println!("Homebrew not found; installing it now (this may prompt for your password)...");
    let script_status = Command::new("/bin/bash")
        .arg("-c")
        .arg("curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh | NONINTERACTIVE=1 /bin/bash")
        .status();
    if script_status.map(|s| s.success()).unwrap_or(false) {
        // Homebrew installs to different prefixes on Apple Silicon vs Intel;
        // neither is on PATH yet in this freshly-spawned process.
        for candidate in ["/opt/homebrew/bin", "/usr/local/bin"] {
            let brew = PathBuf::from(candidate).join("brew");
            if brew.is_file() {
                let path = std::env::var_os("PATH").unwrap_or_default();
                let mut paths: Vec<PathBuf> = vec![PathBuf::from(candidate)];
                paths.extend(std::env::split_paths(&path));
                if let Ok(joined) = std::env::join_paths(paths) {
                    std::env::set_var("PATH", joined);
                }
                break;
            }
        }
    }
    if !command_exists("brew") {
        eprintln!("Error: Homebrew install failed; install it manually from https://brew.sh");
        std::process::exit(1);
    }
}

pub fn ensure_pkg_manager() -> PkgManager {
    let mgr = detect_pkg_manager();
    if mgr == PkgManager::None && os() == Os::Macos {
        bootstrap_homebrew();
        return PkgManager::Brew;
    }
    if mgr == PkgManager::None {
        match os() {
            Os::Windows => eprintln!(
                "Error: winget not found. Update Windows (winget ships with modern Windows 10/11 \
                 via the 'App Installer' package from the Microsoft Store) and re-run."
            ),
            _ => eprintln!(
                "Error: no supported package manager found (looked for apt/dnf/yum/pacman/zypper/apk)."
            ),
        }
        std::process::exit(1);
    }
    mgr
}

/// Installs a package by generic name/id. Windows winget package ids don't
/// match Unix package names 1:1; callers on Windows should pass a winget
/// package id (see `ensure_compiler`/`ensure_cmake` for the ones cforge
/// itself needs). For arbitrary user libraries (`cforge add <lib>`) on
/// Windows, see vcpkg handling in `add_library` instead — winget is for
/// tools, not per-project C/C++ dependencies.
pub fn install_pkg(pkg: &str) {
    match ensure_pkg_manager() {
        PkgManager::Brew => run_or_die("brew", &["install", pkg], "failed to install package"),
        PkgManager::Apt => {
            run_or_die("sudo", &["apt-get", "update", "-y"], "apt-get update failed");
            run_or_die("sudo", &["apt-get", "install", "-y", pkg], "failed to install package");
        }
        PkgManager::Dnf => run_or_die("sudo", &["dnf", "install", "-y", pkg], "failed to install package"),
        PkgManager::Yum => run_or_die("sudo", &["yum", "install", "-y", pkg], "failed to install package"),
        PkgManager::Zypper => run_or_die("sudo", &["zypper", "install", "-y", pkg], "failed to install package"),
        PkgManager::Apk => run_or_die("sudo", &["apk", "add", pkg], "failed to install package"),
        // MSYS2 pacman manages its own prefix and isn't run under sudo (which
        // doesn't exist there); Arch/Linux pacman needs it.
        PkgManager::Pacman if os() == Os::Windows => {
            run_or_die("pacman", &["-Sy", "--noconfirm", pkg], "failed to install package")
        }
        PkgManager::Pacman => run_or_die("sudo", &["pacman", "-Sy", "--noconfirm", pkg], "failed to install package"),
        PkgManager::Winget => run_or_die(
            "winget",
            &["install", "-e", "--id", pkg, "--accept-source-agreements", "--accept-package-agreements"],
            "failed to install package",
        ),
        PkgManager::None => unreachable!("ensure_pkg_manager exits before returning None"),
    }
}

/// Blocks until `xcode-select -p` reports the Command Line Tools are
/// present, or 30 minutes pass. Shared by `ensure_compiler` (hit
/// mid-`new`/`build` on a fresh machine, where there's no one to ask
/// whether to wait) and `toolchain::install_objc_macos` (the explicit
/// `toolchain install obj_c` path, which does ask via its `wait` flag).
pub fn wait_for_xcode_clt() {
    println!("Waiting for the Command Line Tools install to finish...");
    let mut waited = 0;
    while run_capture("xcode-select", &["-p"]).is_none() {
        std::thread::sleep(std::time::Duration::from_secs(5));
        waited += 5;
        if waited >= 1800 {
            eprintln!("Error: timed out waiting for the Command Line Tools installer.");
            std::process::exit(1);
        }
    }
    println!("Command Line Tools installed.");
}

pub fn ensure_compiler() {
    if command_exists(c_compiler()) {
        return;
    }
    match os() {
        Os::Macos => {
            println!("Xcode Command Line Tools not found; opening the installer...");
            run("xcode-select", &["--install"]);
            if crate::flags::get().dry_run {
                return;
            }
            // A first-time machine hits this mid-`new`/`build`, not via an
            // explicit `toolchain install` — there's no one to ask whether
            // to wait, and making them re-run the same command by hand
            // once the GUI installer finishes is exactly the friction this
            // is supposed to remove. So: always wait.
            wait_for_xcode_clt();
        }
        Os::Linux => {
            println!("C/C++ compiler not found; installing...");
            match ensure_pkg_manager() {
                PkgManager::Apt => {
                    run_or_die("sudo", &["apt-get", "update", "-y"], "apt-get update failed");
                    run_or_die("sudo", &["apt-get", "install", "-y", "build-essential"], "compiler install failed");
                }
                PkgManager::Dnf => run_or_die("sudo", &["dnf", "install", "-y", "gcc", "gcc-c++", "make"], "compiler install failed"),
                PkgManager::Yum => run_or_die("sudo", &["yum", "install", "-y", "gcc", "gcc-c++", "make"], "compiler install failed"),
                PkgManager::Pacman => run_or_die("sudo", &["pacman", "-Sy", "--noconfirm", "base-devel"], "compiler install failed"),
                PkgManager::Zypper => run_or_die("sudo", &["zypper", "install", "-y", "gcc", "gcc-c++", "make"], "compiler install failed"),
                PkgManager::Apk => run_or_die("sudo", &["apk", "add", "build-base"], "compiler install failed"),
                _ => unreachable!(),
            }
        }
        Os::Windows => {
            if detect_pkg_manager() == PkgManager::Pacman {
                println!("C/C++ compiler not found; installing the MinGW-w64 toolchain via MSYS2/pacman...");
                install_pkg("mingw-w64-x86_64-toolchain");
            } else {
                println!("C/C++ compiler not found; installing LLVM (clang) via winget...");
                ensure_pkg_manager();
                install_pkg("LLVM.LLVM");
            }
            ensure_ninja();
        }
        Os::Unknown => {
            eprintln!("Error: unsupported OS ({}); install a C/C++ compiler manually.", std::env::consts::OS);
            std::process::exit(1);
        }
    }
}

pub fn ensure_cmake() {
    if command_exists("cmake") {
        return;
    }
    println!("cmake not found; installing...");
    match os() {
        Os::Windows => install_pkg("Kitware.CMake"),
        _ => install_pkg("cmake"),
    }
}

/// Windows has no default make-equivalent that works without a full Visual
/// Studio install, so cforge drives CMake's Ninja generator there instead
/// (paired with clang from `ensure_compiler`). Ninja is a single small
/// binary, unlike a multi-GB Visual Studio Build Tools install.
/// Package name for "ninja" isn't uniform: apt/dnf/yum/zypper/apk call it
/// `ninja-build`, everyone else (brew, pacman, winget) calls it `ninja`
/// (winget's id is `Ninja-build.Ninja`).
pub fn ensure_ninja() {
    if command_exists("ninja") {
        return;
    }
    println!("ninja not found; installing...");
    match detect_pkg_manager() {
        PkgManager::Apt | PkgManager::Dnf | PkgManager::Yum | PkgManager::Zypper | PkgManager::Apk => {
            install_pkg("ninja-build")
        }
        PkgManager::Winget => install_pkg("Ninja-build.Ninja"),
        _ => install_pkg("ninja"),
    }
}

/// Windows C/C++ library management goes through vcpkg rather than winget:
/// winget installs applications/tools (cmake, clang, ninja), not per-project
/// dev dependencies with CMake integration. vcpkg is the standard way to get
/// that on Windows and plugs into CMake via a toolchain file.
pub fn vcpkg_root() -> PathBuf {
    if let Ok(v) = std::env::var("VCPKG_ROOT") {
        return PathBuf::from(v);
    }
    home_dir().join(".cforge").join("vcpkg")
}

pub fn vcpkg_exe() -> PathBuf {
    let base = vcpkg_root();
    if os() == Os::Windows { base.join("vcpkg.exe") } else { base.join("vcpkg") }
}

pub fn vcpkg_toolchain_file() -> PathBuf {
    vcpkg_root().join("scripts").join("buildsystems").join("vcpkg.cmake")
}

/// vcpkg's default triplet for a 64-bit Windows build. Doesn't account for
/// ARM64 Windows or a user-configured custom triplet — a real gap, but one
/// that needs a `--triplet` flag threaded through `add`/`build` to fix
/// properly rather than a guess here.
pub fn vcpkg_triplet() -> &'static str {
    "x64-windows"
}

pub fn ensure_vcpkg() {
    if vcpkg_exe().is_file() {
        return;
    }
    if !command_exists("git") {
        println!("git not found; installing via winget...");
        install_pkg("Git.Git");
    }
    let root = vcpkg_root();
    println!("vcpkg not found; bootstrapping into {}...", root.display());
    run_or_die(
        "git",
        &["clone", "https://github.com/microsoft/vcpkg.git", root.to_str().unwrap_or_default()],
        "failed to clone vcpkg",
    );
    let bootstrap = root.join("bootstrap-vcpkg.bat");
    run_or_die(bootstrap.to_str().unwrap_or_default(), &[], "failed to bootstrap vcpkg");
}
