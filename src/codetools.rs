//! `cforge format`/`lint`/`compdb`: thin wrappers around clang-format,
//! clang-tidy, and `cmake -DCMAKE_EXPORT_COMPILE_COMMANDS=ON`.
use crate::config::Config;
use crate::platform::{self, command_exists, ensure_cmake, install_pkg};
use std::fs;
use std::path::{Path, PathBuf};

const SOURCE_EXTS: &[&str] = &["c", "cpp", "m", "mm", "h", "hpp"];

/// Default roots when no paths are given: the configured source
/// directories plus the header directory. Reading `.cforge.toml` rather
/// than hardcoding `C`/`CPP`/... means `cforge format` on a project with a
/// custom layout formats its files instead of silently finding none.
fn default_roots() -> Vec<PathBuf> {
    let cfg = Config::load();
    ["c", "cpp", "obj_c", "obj_cpp"]
        .iter()
        .map(|l| PathBuf::from(cfg.src_dir(l)))
        .chain(std::iter::once(PathBuf::from(&cfg.paths.headers)))
        .collect()
}

/// Recursive: app-style targets keep their sources in a subdirectory of
/// the language dir (`CPP/mygame/*.cpp`, see cmake.rs's `add_lang_apps`),
/// and a single-level `read_dir` skipped every one of them — `cforge
/// format` quietly formatted only the standalone single-file targets.
fn collect_into(root: &Path, files: &mut Vec<PathBuf>) {
    if root.is_file() {
        files.push(root.to_path_buf());
        return;
    }
    let Ok(entries) = fs::read_dir(root) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_into(&path, files);
        } else if path.extension().and_then(|e| e.to_str()).is_some_and(|e| SOURCE_EXTS.contains(&e)) {
            files.push(path);
        }
    }
}

fn collect_source_files(paths: &[String]) -> Vec<PathBuf> {
    let roots: Vec<PathBuf> =
        if paths.is_empty() { default_roots() } else { paths.iter().map(PathBuf::from).collect() };

    let mut files = Vec::new();
    for root in &roots {
        collect_into(root, &mut files);
    }
    files.sort();
    files.dedup();
    files
}

pub fn compdb() {
    ensure_cmake();
    let build_dir = &std::path::PathBuf::from(Config::load().build_dir());
    platform::create_dir_all(build_dir).ok();
    platform::run_or_die_in(
        build_dir,
        "cmake",
        &["..", "-DCMAKE_EXPORT_COMPILE_COMMANDS=ON"],
        "cmake configure failed",
    );
    if crate::flags::get().dry_run {
        return;
    }
    let generated = build_dir.join("compile_commands.json");
    if generated.is_file() {
        fs::copy(&generated, "compile_commands.json").unwrap_or_else(|e| {
            eprintln!("Error: could not copy compile_commands.json: {e}");
            std::process::exit(1);
        });
    }
    platform::status("Wrote compile_commands.json");
}

pub fn format(paths: &[String]) {
    if !command_exists("clang-format") {
        install_pkg("clang-format");
    }
    let files = collect_source_files(paths);
    if files.is_empty() {
        platform::status("No source files found to format.");
        return;
    }
    let f = crate::flags::get();
    let mut args = vec!["-i".to_string()];
    args.extend(files.iter().map(|p| p.display().to_string()));
    if f.verbose >= 2 || f.dry_run {
        println!("+ clang-format {}", args.join(" "));
    }
    if f.dry_run {
        return;
    }
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    if !platform::run("clang-format", &arg_refs) {
        std::process::exit(1);
    }
    platform::status("Formatted.");
}

pub fn lint(paths: &[String]) {
    if !command_exists("clang-tidy") {
        install_pkg("clang-tidy");
    }
    if !Path::new("compile_commands.json").is_file() {
        compdb();
    }
    let files = collect_source_files(paths);
    if files.is_empty() {
        platform::status("No source files found to lint.");
        return;
    }
    let mut ok = true;
    for file in &files {
        let path_str = file.display().to_string();
        if !platform::run("clang-tidy", &["-p", ".", &path_str]) {
            ok = false;
        }
    }
    if !ok {
        std::process::exit(1);
    }
}
