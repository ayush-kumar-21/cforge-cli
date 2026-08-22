//! Objective-C++ templates. Real Obj-C++ code keeps Foundation at the
//! boundary (an app's `main`, UI glue) and plain C++ everywhere else —
//! mixing NSObject types into a library's public API is generally
//! considered bad practice for portability/interop. So `lib`,
//! `header-lib`, and `test` reuse `cpp.rs`'s content verbatim (valid as-is
//! in a .mm file — Objective-C++ is a strict superset of C++), and only
//! `cli`/`server`, which have a `main`, add the Objective-C flavor
//! (@autoreleasepool, NSLog) real Obj-C++ entrypoints have. See
//! `templates/mod.rs`.
use super::{cpp, scaffold_files};
use crate::config::Config;
use std::path::Path;

const CLI_MAIN: &str = r#"#import "commands.hpp"
#import <Foundation/Foundation.h>
#include <iostream>
#include <vector>

int main(int argc, char** argv) {
    @autoreleasepool {
        if (argc < 2) {
            std::cerr << "Usage: " << argv[0] << " <command> [args...]\n";
            return 1;
        }
        std::vector<std::string> args(argv + 2, argv + argc);
        return run_command(argv[1], args);
    }
}
"#;

const CLI_COMMANDS_HPP: &str = r#"#pragma once
#include <string>
#include <vector>

// A tiny command-line dispatcher: each Command has a name and a
// description. Add a new command by extending kCommands in commands.mm
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

const CLI_COMMANDS_MM: &str = r#"#import "commands.hpp"
#import <Foundation/Foundation.h>
#include <iostream>

namespace {

// NSLog here (rather than std::cout, like the plain C++ template uses) is
// the point of this file being .mm instead of .cpp: bridging a std::string
// into Foundation and back is exactly what Objective-C++ is for.
int cmd_greet(const std::vector<std::string>& args) {
    std::string who = args.empty() ? "world" : args[0];
    NSLog(@"Hello, %s!", who.c_str());
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

fn scaffold_cli(cfg: &Config, project_name: &str) {
    let dir = Path::new(cfg.src_dir("obj_cpp")).join(project_name);
    scaffold_files(
        &dir,
        &[("main.mm", CLI_MAIN), ("commands.hpp", CLI_COMMANDS_HPP), ("commands.mm", CLI_COMMANDS_MM)],
    );
    crate::platform::status(&format!(
        "Scaffolded 'cli' template into {}/ — build with 'cforge build', run with 'cforge run {project_name}'.",
        dir.display()
    ));
}

/// Same public C API as the plain C++ `lib` template (see module docs for
/// why): a compiled library, public header under the configured headers
/// dir, source under Obj_CPP/<name>/, `.cforge_lib` marker for
/// cmake.rs's `add_lang_apps`.
fn scaffold_lib(cfg: &Config, project_name: &str, lib_type: &str) {
    let header_dir = Path::new(&cfg.paths.headers);
    let header = format!(
        "#pragma once\n\n\
         // Public API for the {project_name} library.\n\
         int {project_name}_add(int a, int b);\n\
         int {project_name}_mul(int a, int b);\n"
    );
    scaffold_files(header_dir, &[(&format!("{project_name}.h"), &header)]);

    let dir = Path::new(cfg.src_dir("obj_cpp")).join(project_name);
    let source = format!(
        "#include \"{project_name}.h\"\n\n\
         int {project_name}_add(int a, int b) {{ return a + b; }}\n\
         int {project_name}_mul(int a, int b) {{ return a * b; }}\n"
    );
    let marker = if lib_type == "shared" { "SHARED" } else { "STATIC" };
    scaffold_files(&dir, &[(&format!("{project_name}.mm"), &source), (".cforge_lib", marker)]);

    crate::platform::status(&format!(
        "Scaffolded '{marker}' library '{project_name}' — public header at {}/{project_name}.h, build with 'cforge build'.",
        header_dir.display()
    ));
}

/// Header-only, same `inline` C++ functions as the plain C++ template
/// (see module docs) — the example consumer gets the Obj-C++ flavor.
fn scaffold_header_lib(cfg: &Config, project_name: &str) {
    let header_dir = Path::new(&cfg.paths.headers);
    let header = format!(
        "#pragma once\n\n\
         // {project_name} — header-only library. Everything lives here;\n\
         // there's nothing to link, just #include \"{project_name}.h\".\n\
         inline int {project_name}_add(int a, int b) {{ return a + b; }}\n\
         inline int {project_name}_mul(int a, int b) {{ return a * b; }}\n"
    );
    scaffold_files(header_dir, &[(&format!("{project_name}.h"), &header)]);

    let example_dir = Path::new(cfg.src_dir("obj_cpp")).join(format!("{project_name}_example"));
    let example = format!(
        "#include \"{project_name}.h\"\n\
         #import <Foundation/Foundation.h>\n\n\
         int main() {{\n\
         \x20\x20\x20\x20@autoreleasepool {{\n\
         \x20\x20\x20\x20\x20\x20\x20\x20NSLog(@\"{project_name}_add(2, 3) = %d\", {project_name}_add(2, 3));\n\
         \x20\x20\x20\x20\x20\x20\x20\x20NSLog(@\"{project_name}_mul(2, 3) = %d\", {project_name}_mul(2, 3));\n\
         \x20\x20\x20\x20}}\n\
         \x20\x20\x20\x20return 0;\n\
         }}\n"
    );
    scaffold_files(&example_dir, &[("main.mm", &example)]);

    crate::platform::status(&format!(
        "Scaffolded header-only library '{project_name}' at {}/{project_name}.h — see it used in {}/.",
        header_dir.display(),
        example_dir.display()
    ));
}

/// Same header-only calculator + assert-based test as the plain C++
/// template (see module docs); `*_test` is still auto-registered with
/// CTest by cmake.rs's `add_lang_apps`.
fn scaffold_test(cfg: &Config, project_name: &str) {
    let header_dir = Path::new(&cfg.paths.headers);
    let calc_header = format!(
        "#pragma once\n\n\
         // Small header-only calculator, exercised by both {project_name}\n\
         // and {project_name}_test.\n\
         inline int {project_name}_add(int a, int b) {{ return a + b; }}\n\
         inline int {project_name}_sub(int a, int b) {{ return a - b; }}\n"
    );
    scaffold_files(header_dir, &[(&format!("{project_name}_calc.h"), &calc_header)]);

    let app_dir = Path::new(cfg.src_dir("obj_cpp")).join(project_name);
    let main_mm = format!(
        "#include \"{project_name}_calc.h\"\n\
         #import <Foundation/Foundation.h>\n\n\
         int main() {{\n\
         \x20\x20\x20\x20@autoreleasepool {{\n\
         \x20\x20\x20\x20\x20\x20\x20\x20NSLog(@\"2 + 3 = %d\", {project_name}_add(2, 3));\n\
         \x20\x20\x20\x20}}\n\
         \x20\x20\x20\x20return 0;\n\
         }}\n"
    );
    scaffold_files(&app_dir, &[("main.mm", &main_mm)]);

    let test_dir = Path::new(cfg.src_dir("obj_cpp")).join(format!("{project_name}_test"));
    let test_main = format!(
        "#include \"{project_name}_calc.h\"\n\
         #include <cassert>\n\
         #include <iostream>\n\n\
         int main() {{\n\
         \x20\x20\x20\x20assert({project_name}_add(2, 3) == 5);\n\
         \x20\x20\x20\x20assert({project_name}_sub(5, 3) == 2);\n\
         \x20\x20\x20\x20std::cout << \"all tests passed\\n\";\n\
         \x20\x20\x20\x20return 0;\n\
         }}\n"
    );
    scaffold_files(&test_dir, &[("test_main.mm", &test_main)]);

    crate::platform::status(&format!(
        "Scaffolded 'test' template: app at {}/, tests at {}/ — run with 'cforge test' (or 'cforge run {project_name}_test' directly).",
        app_dir.display(),
        test_dir.display()
    ));
}

/// Reuses cpp.rs's portable BSD-socket implementation verbatim (raw
/// sockets have no Objective-C++ idiom of their own) — only the
/// entrypoint gets the Objective-C flavor (@autoreleasepool).
fn scaffold_server(cfg: &Config, project_name: &str) {
    let dir = Path::new(cfg.src_dir("obj_cpp")).join(project_name);
    let main_mm = "#import \"server.hpp\"\n#import <Foundation/Foundation.h>\n#include <cstdlib>\n\n\
        int main(int argc, char** argv) {\n\
        \x20\x20\x20\x20@autoreleasepool {\n\
        \x20\x20\x20\x20\x20\x20\x20\x20int port = 8080;\n\
        \x20\x20\x20\x20\x20\x20\x20\x20if (argc > 1) {\n\
        \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20port = std::atoi(argv[1]);\n\
        \x20\x20\x20\x20\x20\x20\x20\x20}\n\
        \x20\x20\x20\x20\x20\x20\x20\x20return run_echo_server(port);\n\
        \x20\x20\x20\x20}\n\
        }\n";
    scaffold_files(&dir, &[("main.mm", main_mm), ("server.hpp", cpp::SERVER_HPP), ("server.mm", cpp::SERVER_CPP)]);
    crate::platform::status(&format!(
        "Scaffolded 'server' template (TCP echo server) into {}/ — build with 'cforge build', run with 'cforge run {project_name} -- 8080'.",
        dir.display()
    ));
}

pub fn scaffold(cfg: &Config, template: &str, project_name: &str, lib_type: &str) {
    match template {
        "cli" => scaffold_cli(cfg, project_name),
        "lib" => scaffold_lib(cfg, project_name, lib_type),
        "header-lib" => scaffold_header_lib(cfg, project_name),
        "test" => scaffold_test(cfg, project_name),
        "server" => scaffold_server(cfg, project_name),
        _ => unreachable!("validate_template already rejected anything else"),
    }
}
