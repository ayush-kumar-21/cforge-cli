//! C++ templates: idiomatic modern C++ (RAII, std::string/std::vector,
//! iostream), same shape as the other languages' templates so the five
//! categories (cli/lib/header-lib/test/server) mean the same thing
//! everywhere — see `templates/mod.rs`.
use super::scaffold_files;
use crate::config::Config;
use std::path::Path;

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

fn scaffold_cli(cfg: &Config, project_name: &str) {
    let dir = Path::new(cfg.src_dir("cpp")).join(project_name);
    scaffold_files(
        &dir,
        &[("main.cpp", CLI_MAIN), ("commands.hpp", CLI_COMMANDS_HPP), ("commands.cpp", CLI_COMMANDS_CPP)],
    );
    crate::platform::status(&format!(
        "Scaffolded 'cli' template into {}/ — build with 'cforge build', run with 'cforge run {project_name}'.",
        dir.display()
    ));
}

/// A compiled library (static by default, `--lib-type shared` for a
/// SHARED one): public header under the configured headers dir, source
/// under CPP/<name>/, with a `.cforge_lib` marker file that cmake.rs's
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

    let dir = Path::new(cfg.src_dir("cpp")).join(project_name);
    let source = format!(
        "#include \"{project_name}.h\"\n\n\
         int {project_name}_add(int a, int b) {{ return a + b; }}\n\
         int {project_name}_mul(int a, int b) {{ return a * b; }}\n"
    );
    let marker = if lib_type == "shared" { "SHARED" } else { "STATIC" };
    scaffold_files(&dir, &[(&format!("{project_name}.cpp"), &source), (".cforge_lib", marker)]);

    crate::platform::status(&format!(
        "Scaffolded '{marker}' library '{project_name}' — public header at {}/{project_name}.h, build with 'cforge build'.",
        header_dir.display()
    ));
}

/// Header-only library: everything lives in one public header (`inline`
/// functions, so no separate translation unit or CMake target at all) plus
/// a small example app under CPP/<name>_example/ that includes and uses it.
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

    let example_dir = Path::new(cfg.src_dir("cpp")).join(format!("{project_name}_example"));
    let example = format!(
        "#include \"{project_name}.h\"\n\
         #include <iostream>\n\n\
         int main() {{\n\
         \x20\x20\x20\x20std::cout << \"{project_name}_add(2, 3) = \" << {project_name}_add(2, 3) << \"\\n\";\n\
         \x20\x20\x20\x20std::cout << \"{project_name}_mul(2, 3) = \" << {project_name}_mul(2, 3) << \"\\n\";\n\
         \x20\x20\x20\x20return 0;\n\
         }}\n"
    );
    scaffold_files(&example_dir, &[("main.cpp", &example)]);

    crate::platform::status(&format!(
        "Scaffolded header-only library '{project_name}' at {}/{project_name}.h — see it used in {}/.",
        header_dir.display(),
        example_dir.display()
    ));
}

/// A tiny app plus a hand-rolled, dependency-free test executable
/// (CPP/<name>_test/) that cmake.rs's `add_lang_apps` auto-registers with
/// CTest (any app directory named `*_test`). The logic under test lives in
/// an `inline` header so both the app and the test executable can use it
/// without a library target/target_link_libraries in between.
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

    let app_dir = Path::new(cfg.src_dir("cpp")).join(project_name);
    let main_cpp = format!(
        "#include \"{project_name}_calc.h\"\n\
         #include <iostream>\n\n\
         int main() {{\n\
         \x20\x20\x20\x20std::cout << \"2 + 3 = \" << {project_name}_add(2, 3) << \"\\n\";\n\
         \x20\x20\x20\x20return 0;\n\
         }}\n"
    );
    scaffold_files(&app_dir, &[("main.cpp", &main_cpp)]);

    let test_dir = Path::new(cfg.src_dir("cpp")).join(format!("{project_name}_test"));
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
    scaffold_files(&test_dir, &[("test_main.cpp", &test_main)]);

    crate::platform::status(&format!(
        "Scaffolded 'test' template: app at {}/, tests at {}/ — run with 'cforge test' (or 'cforge run {project_name}_test' directly).",
        app_dir.display(),
        test_dir.display()
    ));
}

pub(super) const SERVER_HPP: &str = r#"#pragma once

// Starts a blocking, single-client-at-a-time TCP echo server on `port` and
// serves forever (or until a fatal socket error). Good enough as a
// starting point; add real concurrency (threads, an event loop) as needed.
int run_echo_server(int port);
"#;

pub(super) const SERVER_CPP: &str = r#"#include "server.hpp"
#include <cstdio>

#ifdef _WIN32
#include <winsock2.h>
#include <ws2tcpip.h>
using socket_t = SOCKET;
constexpr socket_t kInvalidSocket = INVALID_SOCKET;
#else
#include <arpa/inet.h>
#include <netinet/in.h>
#include <sys/socket.h>
#include <unistd.h>
using socket_t = int;
constexpr socket_t kInvalidSocket = -1;
#endif

namespace {

void close_socket(socket_t s) {
#ifdef _WIN32
    closesocket(s);
#else
    close(s);
#endif
}

}  // namespace

int run_echo_server(int port) {
#ifdef _WIN32
    WSADATA wsa;
    if (WSAStartup(MAKEWORD(2, 2), &wsa) != 0) {
        std::fprintf(stderr, "WSAStartup failed\n");
        return 1;
    }
#endif

    socket_t server_fd = socket(AF_INET, SOCK_STREAM, 0);
    if (server_fd == kInvalidSocket) {
        std::fprintf(stderr, "socket() failed\n");
        return 1;
    }

    int reuse = 1;
    setsockopt(server_fd, SOL_SOCKET, SO_REUSEADDR, reinterpret_cast<const char*>(&reuse), sizeof(reuse));

    sockaddr_in addr{};
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = INADDR_ANY;
    addr.sin_port = htons(static_cast<unsigned short>(port));

    if (bind(server_fd, reinterpret_cast<sockaddr*>(&addr), sizeof(addr)) != 0) {
        std::fprintf(stderr, "bind() failed on port %d\n", port);
        close_socket(server_fd);
        return 1;
    }
    if (listen(server_fd, 1) != 0) {
        std::fprintf(stderr, "listen() failed\n");
        close_socket(server_fd);
        return 1;
    }

    std::printf("Listening on port %d -- Ctrl+C to stop.\n", port);
    for (;;) {
        socket_t client_fd = accept(server_fd, nullptr, nullptr);
        if (client_fd == kInvalidSocket) {
            continue;
        }
        char buf[4096];
        for (;;) {
            int n = static_cast<int>(recv(client_fd, buf, sizeof(buf), 0));
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
    let dir = Path::new(cfg.src_dir("cpp")).join(project_name);
    let main_cpp = "#include \"server.hpp\"\n#include <cstdlib>\n\n\
        int main(int argc, char** argv) {\n\
        \x20\x20\x20\x20int port = 8080;\n\
        \x20\x20\x20\x20if (argc > 1) {\n\
        \x20\x20\x20\x20\x20\x20\x20\x20port = std::atoi(argv[1]);\n\
        \x20\x20\x20\x20}\n\
        \x20\x20\x20\x20return run_echo_server(port);\n\
        }\n";
    scaffold_files(&dir, &[("main.cpp", main_cpp), ("server.hpp", SERVER_HPP), ("server.cpp", SERVER_CPP)]);
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
