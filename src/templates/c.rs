//! C templates: plain C99 (structs + function pointers instead of
//! classes, printf/stdio instead of iostream) — see `templates/mod.rs`.
use super::scaffold_files;
use crate::config::Config;
use std::path::Path;

const CLI_MAIN: &str = r#"#include "commands.h"
#include <stdio.h>

int main(int argc, char** argv) {
    if (argc < 2) {
        fprintf(stderr, "Usage: %s <command> [args...]\n", argv[0]);
        return 1;
    }
    return run_command(argv[1], argc - 2, argv + 2);
}
"#;

const CLI_COMMANDS_H: &str = r#"#pragma once

// A tiny command-line dispatcher: each Command has a name, a description,
// and a handler. Add a new command by extending kCommands in commands.c.
typedef int (*CommandFn)(int argc, char** argv);

typedef struct {
    const char* name;
    const char* description;
    CommandFn fn;
} Command;

// Runs the named command with the given arguments. Returns the process
// exit code (0 on success, 1 for an unknown command).
int run_command(const char* name, int argc, char** argv);
"#;

const CLI_COMMANDS_C: &str = r#"#include "commands.h"
#include <stdio.h>
#include <string.h>

static int cmd_greet(int argc, char** argv) {
    const char* who = argc > 0 ? argv[0] : "world";
    printf("Hello, %s!\n", who);
    return 0;
}

static int cmd_version(int argc, char** argv) {
    (void)argc;
    (void)argv;
    printf("0.1.0\n");
    return 0;
}

static const Command kCommands[] = {
    {"greet", "Print a greeting. Usage: greet [name]", cmd_greet},
    {"version", "Print the version.", cmd_version},
};
static const int kCommandCount = sizeof(kCommands) / sizeof(kCommands[0]);

int run_command(const char* name, int argc, char** argv) {
    for (int i = 0; i < kCommandCount; i++) {
        if (strcmp(name, kCommands[i].name) == 0) {
            return kCommands[i].fn(argc, argv);
        }
    }
    fprintf(stderr, "Unknown command '%s'. Available commands:\n", name);
    for (int i = 0; i < kCommandCount; i++) {
        fprintf(stderr, "  %s - %s\n", kCommands[i].name, kCommands[i].description);
    }
    return 1;
}
"#;

fn scaffold_cli(cfg: &Config, project_name: &str) {
    let dir = Path::new(cfg.src_dir("c")).join(project_name);
    scaffold_files(&dir, &[("main.c", CLI_MAIN), ("commands.h", CLI_COMMANDS_H), ("commands.c", CLI_COMMANDS_C)]);
    crate::platform::status(&format!(
        "Scaffolded 'cli' template into {}/ — build with 'cforge build', run with 'cforge run {project_name}'.",
        dir.display()
    ));
}

/// A compiled library (static by default, `--lib-type shared` for a
/// SHARED one): public header under the configured headers dir, source
/// under C/<name>/, with a `.cforge_lib` marker file that cmake.rs's
/// `add_lang_apps` reads to `add_library` instead of `add_executable`.
fn scaffold_lib(cfg: &Config, project_name: &str, lib_type: &str) {
    let header_dir = Path::new(&cfg.paths.headers);
    let header = format!(
        "#pragma once\n\n\
         // Public API for the {project_name} library.\n\
         int {project_name}_add(int a, int b);\n\
         int {project_name}_mul(int a, int b);\n"
    );
    scaffold_files(header_dir, &[(&format!("{project_name}.h"), &header)]);

    let dir = Path::new(cfg.src_dir("c")).join(project_name);
    let source = format!(
        "#include \"{project_name}.h\"\n\n\
         int {project_name}_add(int a, int b) {{ return a + b; }}\n\
         int {project_name}_mul(int a, int b) {{ return a * b; }}\n"
    );
    let marker = if lib_type == "shared" { "SHARED" } else { "STATIC" };
    scaffold_files(&dir, &[(&format!("{project_name}.c"), &source), (".cforge_lib", marker)]);

    crate::platform::status(&format!(
        "Scaffolded '{marker}' library '{project_name}' — public header at {}/{project_name}.h, build with 'cforge build'.",
        header_dir.display()
    ));
}

/// Header-only library: `static inline` functions in one public header —
/// the classic single-header C library idiom (stb-style) — plus a small
/// example app under C/<name>_example/ that includes and uses it.
fn scaffold_header_lib(cfg: &Config, project_name: &str) {
    let header_dir = Path::new(&cfg.paths.headers);
    let header = format!(
        "#pragma once\n\n\
         // {project_name} — header-only library. Everything lives here;\n\
         // there's nothing to link, just #include \"{project_name}.h\".\n\
         static inline int {project_name}_add(int a, int b) {{ return a + b; }}\n\
         static inline int {project_name}_mul(int a, int b) {{ return a * b; }}\n"
    );
    scaffold_files(header_dir, &[(&format!("{project_name}.h"), &header)]);

    let example_dir = Path::new(cfg.src_dir("c")).join(format!("{project_name}_example"));
    let example = format!(
        "#include \"{project_name}.h\"\n\
         #include <stdio.h>\n\n\
         int main(void) {{\n\
         \x20\x20\x20\x20printf(\"{project_name}_add(2, 3) = %d\\n\", {project_name}_add(2, 3));\n\
         \x20\x20\x20\x20printf(\"{project_name}_mul(2, 3) = %d\\n\", {project_name}_mul(2, 3));\n\
         \x20\x20\x20\x20return 0;\n\
         }}\n"
    );
    scaffold_files(&example_dir, &[("main.c", &example)]);

    crate::platform::status(&format!(
        "Scaffolded header-only library '{project_name}' at {}/{project_name}.h — see it used in {}/.",
        header_dir.display(),
        example_dir.display()
    ));
}

/// A tiny app plus a hand-rolled, dependency-free test executable
/// (C/<name>_test/) that cmake.rs's `add_lang_apps` auto-registers with
/// CTest (any app directory named `*_test`). The logic under test lives in
/// a `static inline` header so both the app and the test executable can
/// use it without a library target/target_link_libraries in between.
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

    let app_dir = Path::new(cfg.src_dir("c")).join(project_name);
    let main_c = format!(
        "#include \"{project_name}_calc.h\"\n\
         #include <stdio.h>\n\n\
         int main(void) {{\n\
         \x20\x20\x20\x20printf(\"2 + 3 = %d\\n\", {project_name}_add(2, 3));\n\
         \x20\x20\x20\x20return 0;\n\
         }}\n"
    );
    scaffold_files(&app_dir, &[("main.c", &main_c)]);

    let test_dir = Path::new(cfg.src_dir("c")).join(format!("{project_name}_test"));
    let test_main = format!(
        "#include \"{project_name}_calc.h\"\n\
         #include <assert.h>\n\
         #include <stdio.h>\n\n\
         int main(void) {{\n\
         \x20\x20\x20\x20assert({project_name}_add(2, 3) == 5);\n\
         \x20\x20\x20\x20assert({project_name}_sub(5, 3) == 2);\n\
         \x20\x20\x20\x20printf(\"all tests passed\\n\");\n\
         \x20\x20\x20\x20return 0;\n\
         }}\n"
    );
    scaffold_files(&test_dir, &[("test_main.c", &test_main)]);

    crate::platform::status(&format!(
        "Scaffolded 'test' template: app at {}/, tests at {}/ — run with 'cforge test' (or 'cforge run {project_name}_test' directly).",
        app_dir.display(),
        test_dir.display()
    ));
}

pub(super) const SERVER_H: &str = r#"#pragma once

// Starts a blocking, single-client-at-a-time TCP echo server on `port` and
// serves forever (or until a fatal socket error). Good enough as a
// starting point; add real concurrency (threads, an event loop) as needed.
int run_echo_server(int port);
"#;

pub(super) const SERVER_C: &str = r#"#include "server.h"
#include <stdio.h>

#ifdef _WIN32
#include <winsock2.h>
#include <ws2tcpip.h>
typedef SOCKET socket_t;
#define INVALID_SOCK INVALID_SOCKET
#else
#include <arpa/inet.h>
#include <netinet/in.h>
#include <sys/socket.h>
#include <unistd.h>
typedef int socket_t;
#define INVALID_SOCK (-1)
#endif

static void close_socket(socket_t s) {
#ifdef _WIN32
    closesocket(s);
#else
    close(s);
#endif
}

int run_echo_server(int port) {
#ifdef _WIN32
    WSADATA wsa;
    if (WSAStartup(MAKEWORD(2, 2), &wsa) != 0) {
        fprintf(stderr, "WSAStartup failed\n");
        return 1;
    }
#endif

    socket_t server_fd = socket(AF_INET, SOCK_STREAM, 0);
    if (server_fd == INVALID_SOCK) {
        fprintf(stderr, "socket() failed\n");
        return 1;
    }

    int reuse = 1;
    setsockopt(server_fd, SOL_SOCKET, SO_REUSEADDR, (const char*)&reuse, sizeof(reuse));

    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = INADDR_ANY;
    addr.sin_port = htons((unsigned short)port);

    if (bind(server_fd, (struct sockaddr*)&addr, sizeof(addr)) != 0) {
        fprintf(stderr, "bind() failed on port %d\n", port);
        close_socket(server_fd);
        return 1;
    }
    if (listen(server_fd, 1) != 0) {
        fprintf(stderr, "listen() failed\n");
        close_socket(server_fd);
        return 1;
    }

    printf("Listening on port %d -- Ctrl+C to stop.\n", port);
    for (;;) {
        socket_t client_fd = accept(server_fd, NULL, NULL);
        if (client_fd == INVALID_SOCK) {
            continue;
        }
        char buf[4096];
        for (;;) {
            int n = (int)recv(client_fd, buf, sizeof(buf), 0);
            if (n <= 0) {
                break;
            }
            send(client_fd, buf, n, 0);
        }
        close_socket(client_fd);
    }
}
"#;

fn scaffold_server(cfg: &Config, project_name: &str) {
    let dir = Path::new(cfg.src_dir("c")).join(project_name);
    let main_c = "#include \"server.h\"\n#include <stdlib.h>\n\n\
        int main(int argc, char** argv) {\n\
        \x20\x20\x20\x20int port = 8080;\n\
        \x20\x20\x20\x20if (argc > 1) {\n\
        \x20\x20\x20\x20\x20\x20\x20\x20port = atoi(argv[1]);\n\
        \x20\x20\x20\x20}\n\
        \x20\x20\x20\x20return run_echo_server(port);\n\
        }\n";
    // `<string.h>` (memset) is pulled in by whichever TU needs it — kept
    // out of server.h since it's an implementation detail, not part of
    // the public API.
    let server_c = format!("#include <string.h>\n{SERVER_C}");
    scaffold_files(&dir, &[("main.c", main_c), ("server.h", SERVER_H), ("server.c", &server_c)]);
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
