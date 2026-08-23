//! `cforge new-<lang>-app <name> --template <template>` (or `cforge new
//! <name> --lang <lang> --template <template>`) — scaffolds a real,
//! working multi-file starter program into the project, as an app
//! directory (see cmake.rs's `add_lang_apps` / build.rs's
//! `TargetFile::is_app`): several source files linked into one
//! executable, not one-file-one-executable like a plain `cforge generate`.
//!
//! Every one of cforge's CMake languages (c, cpp, obj_c, obj_cpp) gets its
//! own native implementation of the same five real-world categories
//! (cookiecutter-cpp-lib/-app, cmake-init, etc. all ship these): an app
//! (cli), a compiled library (lib), a header-only library (header-lib), a
//! unit-tested app (test), and a minimal network app (server) — see the
//! `c`/`cpp`/`objc`/`objcpp` submodules. `rust` is separate: cforge
//! doesn't build Rust via CMake, `new-rust-app` just hands off to cargo,
//! so its templates use cargo/crates.io idioms instead (clap, std::net,
//! `#[inline]`, cargo's own test harness).
//!
//! The C-family templates are deliberately dependency-free (no `cforge
//! add` needed first) so `cforge build` works out of the box on
//! macOS/Linux/Windows — a template that fails to build on a fresh machine
//! defeats the point.
mod c;
mod cpp;
mod objc;
mod objcpp;
pub mod rust;

use crate::config::Config;
use crate::platform;
use std::io::IsTerminal;
use std::path::Path;

pub const TEMPLATES: &[&str] = &["cli", "lib", "header-lib", "test", "server"];
pub const LIB_TYPES: &[&str] = &["static", "shared"];

pub fn validate_template(name: &str) {
    if !TEMPLATES.contains(&name) {
        crate::usage_error(&format!("unknown --template '{name}' (expected: {})", TEMPLATES.join("|")));
    }
}

pub fn validate_lib_type(kind: &str) {
    if !LIB_TYPES.contains(&kind) {
        crate::usage_error(&format!("unknown --lib-type '{kind}' (expected: {})", LIB_TYPES.join("|")));
    }
}

/// Creates `dir` and writes each `(filename, content)` pair into it (via
/// `write_new`, so hand-edited files are never clobbered). Every language
/// submodule's scaffold_* functions are built out of one or two calls to
/// this — a template is just "these files go in this directory."
fn scaffold_files(dir: &Path, files: &[(&str, &str)]) {
    make_dir(dir);
    for (name, content) in files {
        platform::write_new(&dir.join(name), content);
    }
}

fn make_dir(dir: &Path) {
    platform::create_dir_all(dir).unwrap_or_else(|e| {
        eprintln!("Error: could not create {}: {e}", dir.display());
        std::process::exit(1);
    });
}

/// Arrow-key language picker for a bare `cforge new <name>` (no --lang, no
/// --template): the npm-create-style flow `new-<lang>-app` already has,
/// minus the language being fixed by which command you typed — here it's
/// also a menu. Single-select only, matching cforge's one-language-per-
/// project model; there is no multi-select variant of this prompt.
///
/// Options come from `all_langs()`, so the menu is exactly the languages
/// this platform can build — obj_c/obj_cpp only appear where
/// `objc_capable_platform()` is true. Rust is deliberately absent: cforge
/// doesn't build Rust projects itself, `new-rust-app` is a separate,
/// non-CMake path.
///
/// Same gating as `prompt_choice` below: never blocks where nothing could
/// answer (no tty, --quiet, --dry-run) — `None` means "fall back to the
/// existing default", same as a cancelled prompt.
pub fn prompt_language() -> Option<String> {
    let flags = crate::flags::get();
    let interactive = std::io::stdin().is_terminal() && !flags.quiet && !flags.dry_run;
    if !interactive {
        return None;
    }
    let langs = crate::project::all_langs();
    let idx =
        dialoguer::Select::new().with_prompt("Select a language").items(langs).default(0).interact_opt().ok()??;
    Some(langs[idx].to_string())
}

/// Arrow-key template picker for `cforge new-<lang>-app`: npm-create-style
/// — the language is picked by which command you ran, then this picks a
/// template from a menu. Returns (template, lib_type); template is `None`
/// for "no template" (a bare project, same as omitting --template).
///
/// Same gating as toolchain.rs's compiler picker: never blocks where
/// nothing could answer (no tty, --quiet, --dry-run) — falls back to no
/// template silently. Esc/Ctrl+C during the prompt is treated the same way.
pub fn prompt_choice() -> (Option<String>, String) {
    let flags = crate::flags::get();
    let interactive = std::io::stdin().is_terminal() && !flags.quiet && !flags.dry_run;
    if !interactive {
        return (None, "static".to_string());
    }

    let mut items: Vec<&str> = vec!["none — bare project, no starter files"];
    items.extend(TEMPLATES);
    let Some(idx) = dialoguer::Select::new()
        .with_prompt("Select a starter template")
        .items(&items)
        .default(0)
        .interact_opt()
        .ok()
        .flatten()
    else {
        return (None, "static".to_string());
    };
    if idx == 0 {
        return (None, "static".to_string());
    }
    let template = TEMPLATES[idx - 1].to_string();

    let lib_type = if template == "lib" {
        let k = dialoguer::Select::new()
            .with_prompt("Library type")
            .items(LIB_TYPES)
            .default(0)
            .interact_opt()
            .ok()
            .flatten()
            .unwrap_or(0);
        LIB_TYPES[k].to_string()
    } else {
        "static".to_string()
    };
    (Some(template), lib_type)
}

/// Scaffolds `template` into the project for the given language
/// (c|cpp|obj_c|obj_cpp), enabling that language in langs.txt first —
/// otherwise a project created with a different `--lang` would get files
/// that silently never build.
pub fn scaffold(template: &str, project_name: &str, lib_type: &str, lang: &str) {
    crate::project::lang_add(&[lang.to_string()]);
    let cfg = Config::load();
    match lang {
        "c" => c::scaffold(&cfg, template, project_name, lib_type),
        "cpp" => cpp::scaffold(&cfg, template, project_name, lib_type),
        "obj_c" => objc::scaffold(&cfg, template, project_name, lib_type),
        "obj_cpp" => objcpp::scaffold(&cfg, template, project_name, lib_type),
        _ => unreachable!("main.rs only calls this for c/cpp/obj_c/obj_cpp"),
    }
}
