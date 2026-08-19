//! `cforge new <name> --template <template>` — scaffolds a real, working
//! multi-file starter program into the project, as an app directory (see
//! cmake.rs's `add_lang_apps` / build.rs's `TargetFile::is_app`): several
//! source files linked into one executable, not one-file-one-executable
//! like a plain `cforge generate`.
//!
//! Templates are deliberately dependency-free so `cforge build` works out
//! of the box on macOS/Linux/Windows without `cforge add` first — a
//! template that fails to build on a fresh machine defeats the point of a
//! template.
use crate::config::Config;
use crate::platform;
use std::path::Path;

pub const TEMPLATES: &[&str] = &["cli"];

pub fn validate_template(name: &str) {
    if !TEMPLATES.contains(&name) {
        crate::usage_error(&format!("unknown --template '{name}' (expected: {})", TEMPLATES.join("|")));
    }
}

const CLI_MAIN: &str = r#"#include "commands.hpp"
#include <iostream>
#include <vector>

int main(int argc, char** argv) {
    if (argc < 2) {
        std::cerr << "Usage: " << argv[0] << " <command> [args...]\n";
        return 1;
    }
    std::vector<std::string> args(argv + 2, argv + argc);
    return run_command(argv[1], args);
}
"#;

const CLI_COMMANDS_HPP: &str = r#"#pragma once
#include <string>
#include <vector>

// A tiny command-line dispatcher: each Command has a name and a
// description. Add a new command by extending kCommands in commands.cpp
// and dispatching to it from run_command().
struct Command {
    std::string name;
    std::string description;
};

extern const std::vector<Command> kCommands;

// Runs the named command with the given arguments. Returns the process
// exit code (0 on success, 1 for an unknown command).
int run_command(const std::string& name, const std::vector<std::string>& args);
"#;

const CLI_COMMANDS_CPP: &str = r#"#include "commands.hpp"
#include <iostream>

namespace {

int cmd_greet(const std::vector<std::string>& args) {
    std::string who = args.empty() ? "world" : args[0];
    std::cout << "Hello, " << who << "!\n";
    return 0;
}

int cmd_version(const std::vector<std::string>&) {
    std::cout << "0.1.0\n";
    return 0;
}

}  // namespace

const std::vector<Command> kCommands = {
    {"greet", "Print a greeting. Usage: greet [name]"},
    {"version", "Print the version."},
};

int run_command(const std::string& name, const std::vector<std::string>& args) {
    if (name == "greet") return cmd_greet(args);
    if (name == "version") return cmd_version(args);

    std::cerr << "Unknown command '" << name << "'. Available commands:\n";
    for (const auto& c : kCommands) {
        std::cerr << "  " << c.name << " - " << c.description << "\n";
    }
    return 1;
}
"#;

/// Scaffolds `template` as a C++ app directory named after the project
/// (`CPP/<project_name>/`) — matching `cargo new`'s convention that a
/// fresh project's own name is also its main binary's name. Existing
/// files are left untouched (same convention as `ffi.rs`'s scaffolding),
/// so re-running this after hand-editing doesn't clobber changes.
pub fn scaffold(template: &str, project_name: &str) {
    let cfg = Config::load();
    let dir = Path::new(cfg.src_dir("cpp")).join(project_name);
    platform::create_dir_all(&dir).unwrap_or_else(|e| {
        eprintln!("Error: could not create {}: {e}", dir.display());
        std::process::exit(1);
    });

    let files: &[(&str, &str)] = match template {
        "cli" => &[("main.cpp", CLI_MAIN), ("commands.hpp", CLI_COMMANDS_HPP), ("commands.cpp", CLI_COMMANDS_CPP)],
        _ => unreachable!("validate_template already rejected anything else"),
    };
    for (filename, content) in files {
        let path = dir.join(filename);
        if !path.exists() {
            platform::write_file(&path, content);
        }
    }
    platform::status(&format!(
        "Scaffolded '{template}' template into {}/ — build with 'cforge build', run with 'cforge run {project_name}'.",
        dir.display()
    ));
}
