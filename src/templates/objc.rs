//! Objective-C templates: idiomatic Foundation (NSString/NSArray/NSLog,
//! @autoreleasepool, @interface/@implementation classes) — see
//! `templates/mod.rs`. Deliberately avoids Blocks/GCD (dispatch_once):
//! cforge's obj_c support extends to Linux via GNUstep with caveats
//! already (project.rs's `LangCapability::Caveats`), and those aren't
//! reliably available there, so plain Objective-C keeps every template
//! buildable on both tiers.
use super::{c, scaffold_files};
use crate::config::Config;
use std::path::Path;

/// `mylib` -> `Mylib`: a minimal, valid Objective-C class name derived
/// from the (already alnum/underscore-sanitized) project name.
fn class_name(project_name: &str) -> String {
    let mut chars = project_name.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => "Project".to_string(),
    }
}

const CLI_MAIN: &str = r#"#import "commands.h"

int main(int argc, char *argv[]) {
    @autoreleasepool {
        if (argc < 2) {
            fprintf(stderr, "Usage: %s <command> [args...]\n", argv[0]);
            return 1;
        }
        NSMutableArray<NSString *> *args = [NSMutableArray array];
        for (int i = 2; i < argc; i++) {
            [args addObject:[NSString stringWithUTF8String:argv[i]]];
        }
        return RunCommand([NSString stringWithUTF8String:argv[1]], args);
    }
}
"#;

const CLI_COMMANDS_H: &str = r#"#pragma once
#import <Foundation/Foundation.h>

// Runs the named command with the given arguments. Returns the process
// exit code (0 on success, 1 for an unknown command). Add a new command
// by extending kCommands in commands.m.
//
// Plain NSArray rather than NSArray<NSString *>: lightweight generics are
// a Clang extension, and Objective-C on Linux goes through GNUstep with
// GCC's frontend, which rejects them outright.
int RunCommand(NSString *name, NSArray *args);
"#;

const CLI_COMMANDS_M: &str = r#"#import "commands.h"
#include <string.h>

typedef int (*CommandFn)(NSArray *args);

typedef struct {
    const char *name;
    const char *description;
    CommandFn fn;
} Command;

static int CmdGreet(NSArray *args) {
    NSString *who = args.count > 0 ? args[0] : @"world";
    NSLog(@"Hello, %@!", who);
    return 0;
}

static int CmdVersion(NSArray *args) {
    (void)args;
    printf("0.1.0\n");
    return 0;
}

static const Command kCommands[] = {
    {"greet", "Print a greeting. Usage: greet [name]", CmdGreet},
    {"version", "Print the version.", CmdVersion},
};
static const NSUInteger kCommandCount = sizeof(kCommands) / sizeof(kCommands[0]);

int RunCommand(NSString *name, NSArray *args) {
    const char *cName = name.UTF8String;
    for (NSUInteger i = 0; i < kCommandCount; i++) {
        if (strcmp(cName, kCommands[i].name) == 0) {
            return kCommands[i].fn(args);
        }
    }
    fprintf(stderr, "Unknown command '%s'. Available commands:\n", cName);
    for (NSUInteger i = 0; i < kCommandCount; i++) {
        fprintf(stderr, "  %s - %s\n", kCommands[i].name, kCommands[i].description);
    }
    return 1;
}
"#;

fn scaffold_cli(cfg: &Config, project_name: &str) {
    let dir = Path::new(cfg.src_dir("obj_c")).join(project_name);
    scaffold_files(&dir, &[("main.m", CLI_MAIN), ("commands.h", CLI_COMMANDS_H), ("commands.m", CLI_COMMANDS_M)]);
    crate::platform::status(&format!(
        "Scaffolded 'cli' template into {}/ — build with 'cforge build', run with 'cforge run {project_name}'.",
        dir.display()
    ));
}

/// A compiled library exposing an `NSObject` class (static by default,
/// `--lib-type shared` for a SHARED one): public header under the
/// configured headers dir, source under Obj_C/<name>/, with a
/// `.cforge_lib` marker file that cmake.rs's `add_lang_apps` reads to
/// `add_library` instead of `add_executable`.
fn scaffold_lib(cfg: &Config, project_name: &str, lib_type: &str) {
    let class_name = class_name(project_name);
    let header_dir = Path::new(&cfg.paths.headers);
    let header = format!(
        "#pragma once\n\
         #import <Foundation/Foundation.h>\n\n\
         // Public API for the {project_name} library.\n\
         @interface {class_name} : NSObject\n\
         + (int)add:(int)a to:(int)b;\n\
         + (int)multiply:(int)a by:(int)b;\n\
         @end\n"
    );
    scaffold_files(header_dir, &[(&format!("{project_name}.h"), &header)]);

    let dir = Path::new(cfg.src_dir("obj_c")).join(project_name);
    let source = format!(
        "#import \"{project_name}.h\"\n\n\
         @implementation {class_name}\n\n\
         + (int)add:(int)a to:(int)b {{\n\
         \x20\x20\x20\x20return a + b;\n\
         }}\n\n\
         + (int)multiply:(int)a by:(int)b {{\n\
         \x20\x20\x20\x20return a * b;\n\
         }}\n\n\
         @end\n"
    );
    let marker = if lib_type == "shared" { "SHARED" } else { "STATIC" };
    scaffold_files(&dir, &[(&format!("{project_name}.m"), &source), (".cforge_lib", marker)]);

    crate::platform::status(&format!(
        "Scaffolded '{marker}' library '{project_name}' — public header at {}/{project_name}.h, build with 'cforge build'.",
        header_dir.display()
    ));
}

/// Header-only: Objective-C classes can't have inline method bodies the
/// way a C++ class can, so — same as real Objective-C codebases that need
/// a zero-link-step utility (see e.g. CGGeometry.h-style helpers) — this
/// uses plain `static inline` C functions in the header. Still perfectly
/// importable from a .m file; there's just no Objective-C class to speak
/// of when nothing needs to be linked.
fn scaffold_header_lib(cfg: &Config, project_name: &str) {
    let header_dir = Path::new(&cfg.paths.headers);
    let header = format!(
        "#pragma once\n\n\
         // {project_name} — header-only library. Everything lives here;\n\
         // there's nothing to link, just #import \"{project_name}.h\".\n\
         // Plain C functions (not an Objective-C class): Objective-C has\n\
         // no inline method bodies, so this is the real Obj-C idiom for a\n\
         // zero-link-step helper (compare CGGeometry.h's inline helpers).\n\
         static inline int {project_name}_add(int a, int b) {{ return a + b; }}\n\
         static inline int {project_name}_mul(int a, int b) {{ return a * b; }}\n"
    );
    scaffold_files(header_dir, &[(&format!("{project_name}.h"), &header)]);

    let example_dir = Path::new(cfg.src_dir("obj_c")).join(format!("{project_name}_example"));
    let example = format!(
        "#import \"{project_name}.h\"\n\
         #import <Foundation/Foundation.h>\n\n\
         int main(void) {{\n\
         \x20\x20\x20\x20@autoreleasepool {{\n\
         \x20\x20\x20\x20\x20\x20\x20\x20NSLog(@\"{project_name}_add(2, 3) = %d\", {project_name}_add(2, 3));\n\
         \x20\x20\x20\x20\x20\x20\x20\x20NSLog(@\"{project_name}_mul(2, 3) = %d\", {project_name}_mul(2, 3));\n\
         \x20\x20\x20\x20}}\n\
         \x20\x20\x20\x20return 0;\n\
         }}\n"
    );
    scaffold_files(&example_dir, &[("main.m", &example)]);

    crate::platform::status(&format!(
        "Scaffolded header-only library '{project_name}' at {}/{project_name}.h — see it used in {}/.",
        header_dir.display(),
        example_dir.display()
    ));
}

/// A tiny app plus a hand-rolled, dependency-free test executable
/// (Obj_C/<name>_test/) that cmake.rs's `add_lang_apps` auto-registers
/// with CTest (any app directory named `*_test`). The logic under test
/// lives in a `static inline` header (same reasoning as header-lib above)
/// so both the app and the test executable can use it without a library
/// target/target_link_libraries in between.
fn scaffold_test(cfg: &Config, project_name: &str) {
    let header_dir = Path::new(&cfg.paths.headers);
    let calc_header = format!(
        "#pragma once\n\n\
         // Small header-only calculator, exercised by both {project_name}\n\
         // and {project_name}_test.\n\
         static inline int {project_name}_add(int a, int b) {{ return a + b; }}\n\
         static inline int {project_name}_sub(int a, int b) {{ return a - b; }}\n"
    );
    scaffold_files(header_dir, &[(&format!("{project_name}_calc.h"), &calc_header)]);

    let app_dir = Path::new(cfg.src_dir("obj_c")).join(project_name);
    let main_m = format!(
        "#import \"{project_name}_calc.h\"\n\
         #import <Foundation/Foundation.h>\n\n\
         int main(void) {{\n\
         \x20\x20\x20\x20@autoreleasepool {{\n\
         \x20\x20\x20\x20\x20\x20\x20\x20NSLog(@\"2 + 3 = %d\", {project_name}_add(2, 3));\n\
         \x20\x20\x20\x20}}\n\
         \x20\x20\x20\x20return 0;\n\
         }}\n"
    );
    scaffold_files(&app_dir, &[("main.m", &main_m)]);

    let test_dir = Path::new(cfg.src_dir("obj_c")).join(format!("{project_name}_test"));
    let test_main = format!(
        "#import \"{project_name}_calc.h\"\n\
         #include <assert.h>\n\
         #include <stdio.h>\n\n\
         int main(void) {{\n\
         \x20\x20\x20\x20assert({project_name}_add(2, 3) == 5);\n\
         \x20\x20\x20\x20assert({project_name}_sub(5, 3) == 2);\n\
         \x20\x20\x20\x20printf(\"all tests passed\\n\");\n\
         \x20\x20\x20\x20return 0;\n\
         }}\n"
    );
    scaffold_files(&test_dir, &[("test_main.m", &test_main)]);

    crate::platform::status(&format!(
        "Scaffolded 'test' template: app at {}/, tests at {}/ — run with 'cforge test' (or 'cforge run {project_name}_test' directly).",
        app_dir.display(),
        test_dir.display()
    ));
}

/// Reuses c.rs's portable BSD-socket implementation verbatim (raw sockets
/// have no Objective-C idiom of their own) — only the entrypoint gets the
/// Objective-C flavor (@autoreleasepool, same as every real Obj-C main).
fn scaffold_server(cfg: &Config, project_name: &str) {
    let dir = Path::new(cfg.src_dir("obj_c")).join(project_name);
    let main_m = "#import \"server.h\"\n#import <Foundation/Foundation.h>\n#include <stdlib.h>\n\n\
        int main(int argc, char *argv[]) {\n\
        \x20\x20\x20\x20@autoreleasepool {\n\
        \x20\x20\x20\x20\x20\x20\x20\x20int port = 8080;\n\
        \x20\x20\x20\x20\x20\x20\x20\x20if (argc > 1) {\n\
        \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20port = atoi(argv[1]);\n\
        \x20\x20\x20\x20\x20\x20\x20\x20}\n\
        \x20\x20\x20\x20\x20\x20\x20\x20return run_echo_server(port);\n\
        \x20\x20\x20\x20}\n\
        }\n";
    let server_m = format!("#include <string.h>\n{}", c::SERVER_C);
    scaffold_files(&dir, &[("main.m", main_m), ("server.h", c::SERVER_H), ("server.m", &server_m)]);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_name_capitalizes_first_letter() {
        assert_eq!(class_name("mylib"), "Mylib");
        assert_eq!(class_name("Already"), "Already");
    }
}
