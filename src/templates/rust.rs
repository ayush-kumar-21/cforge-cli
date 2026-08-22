//! `cforge new-rust-app <name> --template <template>` — unlike the
//! c/cpp/obj_c/obj_cpp templates (module docs in `templates/mod.rs`),
//! cforge doesn't build Rust itself; this just drives `cargo new` (and
//! `cargo add` for `cli`'s dependency) and writes idiomatic Rust source on
//! top. So the five categories mean the same *thing* here but use real
//! Rust/cargo idioms instead of forcing a C/C++ shape onto them:
//!  - `cli` uses clap (the de facto standard for real Rust CLIs) — Rust
//!    deps are normal and cheap (`cargo build` resolves them), unlike the
//!    C-family templates' deliberate dependency-freeness.
//!  - `header-lib`: Rust has no header files at all. The closest real
//!    equivalent to "a library with nothing to link, just include it" is
//!    `#[inline]` on every function — cross-crate inlining, same effect.
//!  - `test` uses `#[cfg(test)]`/`#[test]` (built into cargo, no CTest
//!    wiring needed the way C/C++ needs).
//!  - `server` uses `std::net` (stdlib, zero deps) rather than tokio, to
//!    keep it in the same "just works" spirit as the C-family server.
use crate::platform;
use std::io::IsTerminal;
use std::path::Path;
use std::process::Command;

pub const TEMPLATES: &[&str] = &["cli", "lib", "header-lib", "test", "server"];

pub fn validate_template(name: &str) {
    if !TEMPLATES.contains(&name) {
        crate::usage_error(&format!("unknown --template '{name}' (expected: {})", TEMPLATES.join("|")));
    }
}

/// Same gating/behavior as `templates::prompt_choice` for the C-family
/// languages: never blocks where nothing could answer, falls back to no
/// template (a bare `cargo new`) silently.
pub fn prompt_choice() -> Option<String> {
    let flags = crate::flags::get();
    let interactive = std::io::stdin().is_terminal() && !flags.quiet && !flags.dry_run;
    if !interactive {
        return None;
    }
    let mut items: Vec<&str> = vec!["none — bare project (cargo new only)"];
    items.extend(TEMPLATES);
    let idx = dialoguer::Select::new()
        .with_prompt("Select a starter template")
        .items(&items)
        .default(0)
        .interact_opt()
        .ok()
        .flatten()?;
    if idx == 0 {
        return None;
    }
    Some(TEMPLATES[idx - 1].to_string())
}

fn run_cargo(args: &[&str], dir: Option<&Path>) {
    let mut cmd = Command::new("cargo");
    cmd.args(args);
    if let Some(d) = dir {
        cmd.current_dir(d);
    }
    match cmd.status() {
        Ok(s) if s.success() => {}
        Ok(s) => std::process::exit(s.code().unwrap_or(1)),
        Err(e) => {
            eprintln!("Error: could not run cargo: {e}");
            std::process::exit(1);
        }
    }
}

fn cargo_new(name: &str, is_lib: bool) {
    if !platform::command_exists("cargo") {
        eprintln!("Error: cargo not found on PATH — install Rust from https://rustup.rs first.");
        std::process::exit(1);
    }
    if is_lib {
        run_cargo(&["new", name, "--lib"], None);
    } else {
        run_cargo(&["new", name], None);
    }
}

/// `my-lib` -> `my_lib`: Cargo's own rule for turning a package name into
/// its crate identifier, needed here to reference the crate by path in a
/// doctest (`{ident}::add(..)`).
fn crate_ident(project_name: &str) -> String {
    project_name.replace('-', "_")
}

fn scaffold_cli(dir: &Path, project_name: &str) {
    cargo_new(project_name, false);
    run_cargo(&["add", "clap", "--features", "derive"], Some(dir));
    let main_rs = format!(
        "use clap::{{Parser, Subcommand}};\n\n\
         #[derive(Parser)]\n\
         #[command(name = \"{project_name}\", version)]\n\
         struct Cli {{\n\
         \x20\x20\x20\x20#[command(subcommand)]\n\
         \x20\x20\x20\x20command: Commands,\n\
         }}\n\n\
         #[derive(Subcommand)]\n\
         enum Commands {{\n\
         \x20\x20\x20\x20/// Print a greeting\n\
         \x20\x20\x20\x20Greet {{ who: Option<String> }},\n\
         \x20\x20\x20\x20/// Print the version\n\
         \x20\x20\x20\x20Version,\n\
         }}\n\n\
         fn main() {{\n\
         \x20\x20\x20\x20let cli = Cli::parse();\n\
         \x20\x20\x20\x20match cli.command {{\n\
         \x20\x20\x20\x20\x20\x20\x20\x20Commands::Greet {{ who }} => {{\n\
         \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20println!(\"Hello, {{}}!\", who.unwrap_or_else(|| \"world\".to_string()));\n\
         \x20\x20\x20\x20\x20\x20\x20\x20}}\n\
         \x20\x20\x20\x20\x20\x20\x20\x20Commands::Version => println!(\"0.1.0\"),\n\
         \x20\x20\x20\x20}}\n\
         }}\n"
    );
    platform::write_file(&dir.join("src/main.rs"), &main_rs);
}

fn scaffold_lib(dir: &Path, project_name: &str) {
    cargo_new(project_name, true);
    let ident = crate_ident(project_name);
    let lib_rs = format!(
        "//! Public API for the {project_name} library.\n\n\
         /// Adds two integers.\n\
         ///\n\
         /// # Examples\n\
         ///\n\
         /// ```\n\
         /// assert_eq!({ident}::add(2, 3), 5);\n\
         /// ```\n\
         pub fn add(a: i32, b: i32) -> i32 {{\n\
         \x20\x20\x20\x20a + b\n\
         }}\n\n\
         /// Multiplies two integers.\n\
         pub fn mul(a: i32, b: i32) -> i32 {{\n\
         \x20\x20\x20\x20a * b\n\
         }}\n"
    );
    platform::write_file(&dir.join("src/lib.rs"), &lib_rs);
}

/// Rust has no header-only-library concept (no headers at all); the
/// closest real equivalent is `#[inline]` on every function — the
/// compiler is asked to inline these across crate boundaries, same effect
/// as a C/C++ header-only library never needing its own translation unit.
fn scaffold_header_lib(dir: &Path, project_name: &str) {
    cargo_new(project_name, true);
    let lib_rs = format!(
        "//! {project_name} — a tiny, `#[inline]`-everything utility crate\n\
         //! (Rust's closest equivalent to a C/C++ header-only library).\n\n\
         #[inline]\n\
         pub fn add(a: i32, b: i32) -> i32 {{\n\
         \x20\x20\x20\x20a + b\n\
         }}\n\n\
         #[inline]\n\
         pub fn mul(a: i32, b: i32) -> i32 {{\n\
         \x20\x20\x20\x20a * b\n\
         }}\n\n\
         #[cfg(test)]\n\
         mod tests {{\n\
         \x20\x20\x20\x20use super::*;\n\n\
         \x20\x20\x20\x20#[test]\n\
         \x20\x20\x20\x20fn it_adds_and_multiplies() {{\n\
         \x20\x20\x20\x20\x20\x20\x20\x20assert_eq!(add(2, 3), 5);\n\
         \x20\x20\x20\x20\x20\x20\x20\x20assert_eq!(mul(2, 3), 6);\n\
         \x20\x20\x20\x20}}\n\
         }}\n"
    );
    platform::write_file(&dir.join("src/lib.rs"), &lib_rs);
}

/// Idiomatic Rust testing: `#[cfg(test)] mod tests` next to the code it
/// covers, run with `cargo test` — no CTest-style registration needed the
/// way the C-family `test` template requires.
fn scaffold_test(dir: &Path, project_name: &str) {
    let _ = project_name;
    cargo_new(project_name, false);
    let main_rs = "mod calc;\n\n\
        fn main() {\n\
        \x20\x20\x20\x20println!(\"2 + 3 = {}\", calc::add(2, 3));\n\
        \x20\x20\x20\x20println!(\"5 - 3 = {}\", calc::sub(5, 3));\n\
        }\n";
    platform::write_file(&dir.join("src/main.rs"), main_rs);

    let calc_rs = "//! Small calculator module, used by main() and covered by the unit\n\
        //! tests below (`cargo test`).\n\n\
        pub fn add(a: i32, b: i32) -> i32 {\n\
        \x20\x20\x20\x20a + b\n\
        }\n\n\
        pub fn sub(a: i32, b: i32) -> i32 {\n\
        \x20\x20\x20\x20a - b\n\
        }\n\n\
        #[cfg(test)]\n\
        mod tests {\n\
        \x20\x20\x20\x20use super::*;\n\n\
        \x20\x20\x20\x20#[test]\n\
        \x20\x20\x20\x20fn adds() {\n\
        \x20\x20\x20\x20\x20\x20\x20\x20assert_eq!(add(2, 3), 5);\n\
        \x20\x20\x20\x20}\n\n\
        \x20\x20\x20\x20#[test]\n\
        \x20\x20\x20\x20fn subtracts() {\n\
        \x20\x20\x20\x20\x20\x20\x20\x20assert_eq!(sub(5, 3), 2);\n\
        \x20\x20\x20\x20}\n\
        }\n";
    platform::write_file(&dir.join("src/calc.rs"), calc_rs);
}

/// `std::net` (stdlib, zero deps) rather than tokio/async — keeps this in
/// the same "just works, nothing to fetch" spirit as the C-family server.
fn scaffold_server(dir: &Path, project_name: &str) {
    cargo_new(project_name, false);
    let main_rs = r#"use std::io::{Read, Write};
use std::net::TcpListener;

/// Blocking, single-client-at-a-time TCP echo server. Good enough as a
/// starting point; reach for threads or async (tokio) for real concurrency.
fn main() -> std::io::Result<()> {
    let port: u16 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(8080);
    let listener = TcpListener::bind(("0.0.0.0", port))?;
    println!("Listening on port {port} -- Ctrl+C to stop.");

    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(s) => s,
            Err(_) => continue,
        };
        let mut buf = [0u8; 4096];
        loop {
            let n = match stream.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };
            if stream.write_all(&buf[..n]).is_err() {
                break;
            }
        }
    }
    Ok(())
}
"#;
    platform::write_file(&dir.join("src/main.rs"), main_rs);
    let _ = project_name;
}

pub fn scaffold(project_name: &str, template: &str) {
    let dir = Path::new(project_name);
    match template {
        "cli" => scaffold_cli(dir, project_name),
        "lib" => scaffold_lib(dir, project_name),
        "header-lib" => scaffold_header_lib(dir, project_name),
        "test" => scaffold_test(dir, project_name),
        "server" => scaffold_server(dir, project_name),
        _ => unreachable!("validate_template already rejected anything else"),
    }
    platform::status(&format!(
        "Scaffolded '{template}' Rust template into {project_name}/ — build with 'cargo build' (or 'cd {project_name} && cargo run')."
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_ident_replaces_hyphens() {
        assert_eq!(crate_ident("my-lib"), "my_lib");
        assert_eq!(crate_ident("plain"), "plain");
    }
}
