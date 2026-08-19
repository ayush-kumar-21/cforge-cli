//! `cforge new <name> --template <template>` — scaffolds a real, working
//! multi-file starter program into the project, as an app directory (see
//! cmake.rs's `add_lang_apps` / build.rs's `TargetFile::is_app`): several
//! source files linked into one executable, not one-file-one-executable
//! like a plain `cforge generate`.
//!
//! Templates are deliberately dependency-free (no SDL/raylib/etc.) so
//! `cforge build` works out of the box on macOS/Linux/Windows without
//! `cforge add` first — a template that fails to build on a fresh machine
//! defeats the point of a template.
use crate::config::Config;
use crate::platform;
use std::path::Path;

pub const TEMPLATES: &[&str] = &["cli", "game"];

pub fn validate_template(name: &str) {
    if !TEMPLATES.contains(&name) {
        crate::usage_error(&format!("unknown --template '{name}' (expected: {})", TEMPLATES.join("|")));
    }
}

/// Game engine integrations: a real C/C++ graphics library `cforge add`
/// already knows how to install and pkg-config-link (verified end-to-end:
/// installed via brew, resolved via pkg-config, and a program calling its
/// real API — InitWindow/BeginDrawing/EndDrawing — compiled and linked
/// clean). Only ever paired with `--template game`, which it swaps from a
/// zero-dependency stdin loop to an actual windowed game loop. This is
/// deliberately not "Unity/Godot/Unreal integration" — those are entire
/// separate build systems and runtimes, not something a CMake wrapper
/// could meaningfully drive; a linkable C/C++ library is what's real here.
pub const ENGINES: &[&str] = &["raylib"];

pub fn validate_engine(name: &str) {
    if !ENGINES.contains(&name) {
        crate::usage_error(&format!("unknown --engine '{name}' (expected: {})", ENGINES.join("|")));
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

const GAME_MAIN: &str = r#"#include "Game.hpp"

int main() {
    Game game;
    game.run();
    return 0;
}
"#;

const GAME_PLAYER_HPP: &str = r#"#pragma once
#include <string>

// A single actor in the game world with a position and simple movement.
// Kept independent of Game/input/rendering so it can be unit-tested or
// reused (e.g. for an Enemy class) on its own.
class Player {
public:
    explicit Player(std::string name);
    void move(int dx, int dy);
    std::string describe() const;

private:
    std::string name_;
    int x_ = 0;
    int y_ = 0;
};
"#;

const GAME_PLAYER_CPP: &str = r#"#include "Player.hpp"

Player::Player(std::string name) : name_(std::move(name)) {}

void Player::move(int dx, int dy) {
    x_ += dx;
    y_ += dy;
}

std::string Player::describe() const {
    return name_ + " is at (" + std::to_string(x_) + ", " + std::to_string(y_) + ")";
}
"#;

const GAME_GAME_HPP: &str = r#"#pragma once
#include "Player.hpp"

// Drives a minimal turn-based loop over stdin: type a direction (n/s/e/w)
// each turn, or 'q' to quit. This has no external dependency (no SDL,
// raylib, ...) so it builds and runs identically on macOS/Linux/Windows
// with nothing to install first — swap this loop for a real render/input
// backend without touching Player, which is the point of keeping them in
// separate files linked into one binary.
class Game {
public:
    Game();
    void run();

private:
    Player player_;
};
"#;

const GAME_GAME_CPP: &str = r#"#include "Game.hpp"
#include <iostream>

Game::Game() : player_("Hero") {}

void Game::run() {
    std::cout << "A tiny turn-based loop. Move with n/s/e/w, quit with q.\n";
    std::cout << player_.describe() << "\n";

    char move;
    while (std::cin >> move) {
        switch (move) {
            case 'n': player_.move(0, -1); break;
            case 's': player_.move(0, 1); break;
            case 'e': player_.move(1, 0); break;
            case 'w': player_.move(-1, 0); break;
            case 'q': std::cout << "Bye!\n"; return;
            default: std::cout << "Unknown move '" << move << "' (use n/s/e/w/q)\n"; continue;
        }
        std::cout << player_.describe() << "\n";
    }
}
"#;

const RAYLIB_MAIN: &str = r#"#include "Engine.hpp"

int main() {
    Engine engine(640, 480, "cforge game (raylib)");
    engine.run();
    return 0;
}
"#;

const RAYLIB_PLAYER_HPP: &str = r#"#pragma once

// A single controllable actor: position plus raylib input/draw calls.
// Kept independent of Engine's window/frame lifecycle so it could be
// reused for an Enemy class, or unit-tested without a window at all.
class Player {
public:
    Player(float x, float y);
    void handleInput(float dt);
    void draw() const;

private:
    float x_;
    float y_;
    static constexpr float kSpeed = 200.0f;  // pixels per second
    static constexpr float kSize = 32.0f;
};
"#;

const RAYLIB_PLAYER_CPP: &str = r#"#include "Player.hpp"
#include "raylib.h"

Player::Player(float x, float y) : x_(x), y_(y) {}

void Player::handleInput(float dt) {
    float delta = kSpeed * dt;
    if (IsKeyDown(KEY_RIGHT)) x_ += delta;
    if (IsKeyDown(KEY_LEFT)) x_ -= delta;
    if (IsKeyDown(KEY_DOWN)) y_ += delta;
    if (IsKeyDown(KEY_UP)) y_ -= delta;
}

void Player::draw() const {
    DrawRectangle(static_cast<int>(x_ - kSize / 2), static_cast<int>(y_ - kSize / 2),
                  static_cast<int>(kSize), static_cast<int>(kSize), MAROON);
}
"#;

const RAYLIB_ENGINE_HPP: &str = r#"#pragma once
#include "Player.hpp"

// Thin wrapper around raylib's window/frame lifecycle (InitWindow /
// BeginDrawing+EndDrawing / CloseWindow) so main.cpp and Player don't need
// to touch raylib's C API directly. Swap the body of run() for a
// different backend without touching Player at all — that's the point of
// keeping them in separate files linked into one binary.
class Engine {
public:
    Engine(int width, int height, const char* title);
    ~Engine();
    void run();

private:
    Player player_;
};
"#;

const RAYLIB_ENGINE_CPP: &str = r#"#include "Engine.hpp"
#include "raylib.h"

Engine::Engine(int width, int height, const char* title) : player_(width / 2.0f, height / 2.0f) {
    InitWindow(width, height, title);
    SetTargetFPS(60);
}

Engine::~Engine() {
    CloseWindow();
}

void Engine::run() {
    while (!WindowShouldClose()) {
        player_.handleInput(GetFrameTime());

        BeginDrawing();
        ClearBackground(RAYWHITE);
        DrawText("Arrow keys to move, Esc to quit", 10, 10, 20, DARKGRAY);
        player_.draw();
        EndDrawing();
    }
}
"#;

/// Scaffolds `template` as a C++ app directory named after the project
/// (`CPP/<project_name>/`) — matching `cargo new`'s convention that a
/// fresh project's own name is also its main binary's name. Existing
/// files are left untouched (same convention as `ffi.rs`'s scaffolding),
/// so re-running this after hand-editing doesn't clobber changes.
///
/// `engine` only applies to the "game" template (checked by the caller via
/// `validate_engine`/CLI parsing before this runs); when given, it both
/// swaps in the engine-backed file set and installs+links the library
/// through the exact same path `cforge add <lib>` uses — `--template game
/// --engine raylib` needs raylib.h to actually resolve at compile time,
/// not just have plausible-looking #include lines.
pub fn scaffold(template: &str, project_name: &str, engine: Option<&str>) {
    let cfg = Config::load();
    let dir = Path::new(cfg.src_dir("cpp")).join(project_name);
    platform::create_dir_all(&dir).unwrap_or_else(|e| {
        eprintln!("Error: could not create {}: {e}", dir.display());
        std::process::exit(1);
    });

    let files: &[(&str, &str)] = match (template, engine) {
        ("cli", _) => &[("main.cpp", CLI_MAIN), ("commands.hpp", CLI_COMMANDS_HPP), ("commands.cpp", CLI_COMMANDS_CPP)],
        ("game", Some("raylib")) => {
            crate::build::add_library("raylib");
            &[
                ("main.cpp", RAYLIB_MAIN),
                ("Engine.hpp", RAYLIB_ENGINE_HPP),
                ("Engine.cpp", RAYLIB_ENGINE_CPP),
                ("Player.hpp", RAYLIB_PLAYER_HPP),
                ("Player.cpp", RAYLIB_PLAYER_CPP),
            ]
        }
        ("game", _) => {
            &[("main.cpp", GAME_MAIN), ("Player.hpp", GAME_PLAYER_HPP), ("Player.cpp", GAME_PLAYER_CPP), ("Game.hpp", GAME_GAME_HPP), ("Game.cpp", GAME_GAME_CPP)]
        }
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
