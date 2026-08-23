# cforge Manual

Complete reference for `cforge`. For installation and a first-project
walkthrough, see the [README](README.md).

Every command is also documented in the tool itself:

```sh
cforge --help              # full command list
cforge <command> -h        # detailed help for one command
cforge toolchain use -h    # subcommands have their own help too
```

---

## Contents

- [Concepts](#concepts)
- [Project layout](#project-layout)
- [How targets are discovered](#how-targets-are-discovered)
- [Global options](#global-options)
- [Command reference](#command-reference)
  - [Project setup](#project-setup)
  - [Build and run](#build-and-run)
  - [Toolchain](#toolchain)
  - [Languages](#languages)
  - [Standards](#standards)
  - [Dependencies](#dependencies)
  - [Project layout configuration](#project-layout-configuration)
  - [Code tools](#code-tools)
  - [Tool management](#tool-management)
- [Project files](#project-files)
- [Templates](#templates)
- [Rust FFI](#rust-ffi)
- [Recipes](#recipes)
- [Troubleshooting](#troubleshooting)

---

## Concepts

**cforge generates a CMake project and drives it.** `cforge init` writes a
`CMakeLists.txt`; every later command shells out to `cmake`, `ctest`, `cpack`,
or your package manager. There is no proprietary build engine, and the
generated files are ordinary CMake you can read or take with you.

**No configure step.** `cforge build` configures on first use and reuses that
configuration afterwards. `cforge clean --all` forces a fresh configure.

**Convention over configuration.** Source files live in per-language
directories (`C/`, `CPP/`, `Obj_C/`, `Obj_CPP/`), and cforge turns what it
finds there into build targets automatically — nothing to register by hand.

**Languages are opt-in per project.** A new project enables only C unless
`--lang` says otherwise. Enabled languages are recorded in `langs.txt`, and
only those directories are created and built.

**Missing tools install themselves.** A package manager, a C/C++ compiler,
CMake, and Ninja on Windows are installed on demand the first time a command
needs one.

---

## Project layout

A project created with `cforge new myapp --lang c cpp` looks like this:

```
myapp/
├── CMakeLists.txt        generated — regenerate by deleting it and running 'cforge init'
├── .cforge_config.cmake  generated from .cforge.toml — do not edit directly
├── .cforge.toml          directory configuration (only if you changed a path)
├── langs.txt             enabled languages
├── libs.txt              linked libraries
├── toolchain.txt         per-language compiler pins (only if you set one)
├── deps.lock             resolved package names per platform
├── C/                    C sources
├── CPP/                  C++ sources
├── include/              public headers
└── build/                CMake build directory (all artifacts land here)
```

Only the directories for enabled languages are created. `Obj_C/` and
`Obj_CPP/` appear on macOS and Linux when those languages are enabled, never
on Windows. Any of these paths can be changed — see
[Project layout configuration](#project-layout-configuration).

---

## How targets are discovered

Inside a language's source directory, cforge recognizes three shapes:

**1. A standalone source file → one executable.**

```
CPP/tool.cpp        →  target 'tool'
```

**2. A subdirectory of source files → one executable, named after the
directory.** All files sharing the language's extension are compiled and
linked together. This is how multi-file programs work:

```
CPP/myapp/main.cpp
CPP/myapp/util.cpp  →  target 'myapp' (one binary, both files linked)
```

**3. A subdirectory with special markers → a library or a test.**

| Marker | Result |
|---|---|
| A `.cforge_lib` file containing `STATIC` or `SHARED` | Built with `add_library` instead of `add_executable` |
| A directory name ending in `_test` | Also registered with CTest, so `cforge test` runs it |

Both markers are what `--template lib` and `--template test` set up for you.

**Name collisions across languages.** If `C/foo.c` and `CPP/foo.cpp` both
exist, the target name `foo` is ambiguous. Pass the extension to disambiguate:
`cforge build foo.c` or `cforge build foo.cpp`.

---

## Global options

These are recognized anywhere on the command line, before or after the
command word — `cforge -v build foo` and `cforge build foo -v` are identical.

| Option | Effect |
|---|---|
| `-h`, `--help` | Show help. Alone: the global screen. After a command: that command's help |
| `-V`, `--version` | Print the version (`--verbose` adds cmake/compiler/platform info) |
| `-v`, `--verbose` | Verbose output. `-vv` also echoes every command cforge runs |
| `-q`, `--quiet` | Suppress cforge's own status messages. The build tool's own output (compiler errors) is unaffected |
| `--profile <name>` | `debug`, `release`, or `relwithdebinfo`. Maps to `CMAKE_BUILD_TYPE` |
| `--jobs <n>` | Parallel build and test jobs |
| `--no-color` | Disable ANSI color. `NO_COLOR` in the environment does the same |
| `--dry-run` | Print what would happen, including file writes, without doing it |

---

## Command reference

### Project setup

#### `cforge new <name> [options]`

Creates `<name>/`, writes `CMakeLists.txt`, creates the enabled languages'
source directories plus `build/`, and records the languages in `langs.txt`.

**With no `--lang` and no `--template`, run from an interactive terminal**,
this prompts instead — npm-create-style: pick a language, then a template,
from arrow-key menus. Both are single-select; a project is always exactly
one language. This is the same two-step flow `new-<lang>-app` (below) uses,
except the language step is also a menu rather than being fixed by which
command you typed.

Passing `--lang` and/or `--template` explicitly skips the corresponding
prompt. So does anywhere without a terminal to prompt on — a script, CI, a
pipe — which falls back to the old default silently: single language `c`,
no template.

| Option | Meaning |
|---|---|
| `--lang <lang>...` | Languages to enable: `c`, `cpp`, `obj_c`, `obj_cpp`. Default: `c` only |
| `--template <name>` | Scaffold a starter program — see [Templates](#templates) |
| `--lib-type <kind>` | `static` (default) or `shared`. Only meaningful with `--template lib` |
| `--ffi rust` | Scaffold a C ABI boundary plus a Rust `bindings/` crate — see [Rust FFI](#rust-ffi) |

`--template` applies to the first `--lang` given (as typed, or as picked
interactively), or to C if `--lang` was omitted.

```sh
cforge new myapp                # interactive: prompts for language + template
cforge new myapp --lang cpp     # skips the language prompt only
cforge new myapp --lang c cpp   # multiple languages — always explicit, never prompted
cforge new mylib --template lib --lib-type shared
```

Without `--template`, you get the structure and no source code. Add your own
files to `C/` or `CPP/` and they become targets.

#### `cforge new-c-app|new-cpp-app|new-objc-app|new-objcpp-app <name> [options]`

The same thing, with the language fixed by which command you type. If
`--template` is omitted and you are on an interactive terminal, an arrow-key
menu picks one (including `none` for a bare project).

```sh
cforge new-cpp-app myapp                    # menu
cforge new-cpp-app myapp --template cli     # no menu
```

`new-objc-app` and `new-objcpp-app` require macOS; they exit with an error
on Linux and Windows.

#### `cforge new-rust-app <name> [options]`

A plain Rust/Cargo project — **not** a cforge/CMake one. cforge hands off to
`cargo` entirely and does not build the result.

With `--template` (or the arrow-key menu), it drives `cargo new` and scaffolds
idiomatic Rust on top: `cli` (clap), `lib`, `header-lib` (an `#[inline]`
utility crate), `test` (`#[cfg(test)]`), `server` (a `std::net` echo server).
With neither, extra arguments pass straight through to `cargo new`:

```sh
cforge new-rust-app mylib --lib     # → cargo new mylib --lib
```

Requires `cargo` on PATH — install from [rustup.rs](https://rustup.rs).

#### `cforge init`

Sets up cforge in the current directory instead of creating a new one: writes
`CMakeLists.txt` and creates the source directories. Use this on a directory
of existing source files. If `CMakeLists.txt` already exists, it is left
alone.

#### `cforge info`

Prints the project's enabled languages, pinned standards, and linked
libraries.

---

### Build and run

#### `cforge build [target...]`

Configures if needed, then builds. With no arguments, builds every target
across the enabled languages. With target names, builds only those.

```sh
cforge build
cforge build myapp
cforge build foo.c              # disambiguate a cross-language name collision
cforge build --profile release
cforge build --jobs 8
```

#### `cforge run <target> [-- <args>...]`

Builds a single target, then runs it. Everything after `--` is forwarded to
your program rather than consumed by cforge. Your program's exit code becomes
cforge's exit code.

```sh
cforge run myapp
cforge run myapp -- --flag value
```

#### `cforge test [filter]`

Runs `ctest` against `build/` with `--output-on-failure`. A filter maps to
`ctest -R <filter>`, so it is a regular expression matched against test names.

Tests come from directories whose name ends in `_test` (see
[How targets are discovered](#how-targets-are-discovered)) — `--template test`
sets one up.

#### `cforge clean [--all]`

Removes build outputs but keeps the CMake cache, so the next build
reconfigures quickly. `--all` removes `build/` entirely, forcing a full
reconfigure — the right move when the cache itself is the problem.

#### `cforge install [--prefix <path>]`

Runs the CMake install step, placing built executables in `<prefix>/bin`.
With no `--prefix`, CMake's own default applies (`/usr/local` on macOS and
Linux). Requires a prior `cforge build`.

#### `cforge package [--format <fmt>]`

Packages the project with CPack. Formats: `tgz` (default), `zip`, `dmg`,
`deb`, `rpm`. Requires a prior `cforge build`.

---

### Toolchain

#### `cforge doctor`

The first thing to run when something is wrong. Checks for a package manager,
CMake, and a compiler for each enabled language, then validates the project's
pinned standards against what the installed compiler actually supports.

#### `cforge toolchain list`

Shows the detected `cc`/`c++` and any per-language overrides from
`toolchain.txt`.

#### `cforge toolchain install <lang> [options]`

Installs a compiler or runtime for a language.

| Option | Meaning |
|---|---|
| `--compiler <name>` | `gcc`, `clang`, `mingw`, `msvc`, or `apple-clang` (C/C++ only) |
| `--version <v>` | Compiler version, where the package manager supports it |
| `--wait` | Objective-C: block until the Command Line Tools installer finishes |

```sh
cforge toolchain install cpp --compiler gcc --version 14
cforge toolchain install obj_c                         # macOS Command Line Tools
```

#### `cforge toolchain use <lang> <compiler>`

Pins a language to a specific compiler for this project. Recorded in
`toolchain.txt` and passed to CMake as `-DCMAKE_<LANG>_COMPILER` on the next
build.

#### `cforge toolchain default`

Clears `toolchain.txt`, returning to system-default compilers.

#### `cforge toolchain remove <compiler>`

Uninstalls a compiler through the OS package manager. Refuses if
`toolchain.txt` still names it for any language — run `toolchain use` or
`toolchain default` first.

---

### Languages

Enabled languages live in `langs.txt` and control which source directories are
created and built.

```sh
cforge lang add cpp obj_c     # enable languages
cforge lang remove obj_c      # disable
cforge lang list              # show what is enabled
```

Valid values: `c`, `cpp`, `obj_c`, `obj_cpp`. Objective-C and Objective-C++
are rejected anywhere but macOS.

---

### Standards

Standards map to `CMAKE_<LANG>_STANDARD`. The keys are spelled to match:
`c`, `cxx`, `objc`, `objcpp`.

```sh
cforge std list               # current values, plus what 'latest' resolves to
cforge std set cxx 20         # a specific year
cforge std set --all latest   # pin every supported language to its newest
```

`latest` is resolved by probing your installed compiler, not hardcoded — so it
means the newest standard your compiler actually accepts. New projects start
at whatever the generated `CMakeLists.txt` defaults to; `std set` is the only
way to change it.

---

### Dependencies

#### `cforge add <library>...`

Installs a library through the platform's package manager and links it into
the build. On macOS and Linux this uses Homebrew, apt, dnf, pacman, zypper, or
apk plus pkg-config; on Windows it uses vcpkg, set up automatically on first
use. The resolved package name is recorded in `deps.lock`.

```sh
cforge add openssl
cforge add sqlite3 zlib
```

#### `cforge remove <library>...`

Unlinks a library from the build. Does not uninstall the system package.

#### `cforge list`

Lists the libraries currently linked (the contents of `libs.txt`).

#### `cforge search <name>`

Searches the OS package manager by name — useful when `cforge add` cannot
resolve a library and you need the real package name for your platform.

#### `cforge deps show`

Shows each library in `libs.txt` alongside the exact package name it resolved
to on this platform, from `deps.lock`.

#### `cforge deps sync`

Reinstalls every library in `libs.txt`, preferring the package name pinned in
`deps.lock` for this platform and falling back to re-resolving if that pin no
longer verifies. **This is the command for a fresh checkout or a CI job** —
run it once instead of repeating `cforge add` for every dependency.

---

### Project layout configuration

Source directory paths are stored in `.cforge.toml` and mirrored into the
generated `.cforge_config.cmake`.

```sh
cforge config show                 # current paths
cforge config set cpp_src src      # move C++ sources to src/
cforge config reset                # back to defaults
```

| Key | Default |
|---|---|
| `c_src` | `C` |
| `cpp_src` | `CPP` |
| `obj_c_src` | `Obj_C` |
| `obj_cpp_src` | `Obj_CPP` |
| `headers` | `include` |
| `build` | `build` |

Changing a path does not move existing files — move them yourself, then
rebuild.

---

### Code tools

#### `cforge generate <name> [options]`

Adds to the current project by name — the command for creating a single file,
or a whole library (or app, or test target), inside a project that already
exists.

| Option | Meaning |
|---|---|
| `--lang <lang>` | `c`, `cpp`, `obj_c`, or `obj_cpp`. Inferred when the project has exactly one language enabled; required otherwise |
| `--template <name>` | One of the five templates in [Templates](#templates) — omit for a plain empty file |
| `--lib-type <kind>` | `static` (default) or `shared`. Only meaningful with `--template lib` |

```sh
cforge generate binarySearch --lang cpp   # a single empty file
cforge generate mylib --template lib      # a library, --lang inferred
```

`--template` reuses the exact scaffolding `cforge new --template` uses — the
only difference is the target project already exists rather than being
created fresh. `--lang` is optional on both forms whenever the project has
exactly one language enabled, which is the default now that a project is
single-language unless `--lang` said otherwise at creation time. With zero or
several languages enabled, `--lang` is required and cforge says so rather
than guessing.

#### `cforge format [path...]`

Runs `clang-format -i` over the given paths, or all source directories if none
are given.

#### `cforge lint [path...]`

Runs `clang-tidy` over the given paths, or all source directories if none are
given.

#### `cforge compdb`

Writes `compile_commands.json` at the project root. Run this once so your
editor or language server (clangd, VS Code, CLion) sees the project's include
paths and flags.

---

### Tool management

```sh
cforge self-update       # update cforge to the latest release
cforge self-uninstall    # remove cforge and its PATH entry
```

Uninstalling touches only the tool, never projects built with it. If cforge is
too broken to run `self-uninstall`, use `./uninstall.sh` (macOS/Linux) or
`.\uninstall.ps1` (Windows) from a clone of the repository.

---

## Project files

| File | Written by | Safe to edit |
|---|---|---|
| `CMakeLists.txt` | `cforge init` / `cforge new` | Yes, but regenerating overwrites it |
| `.cforge_config.cmake` | Generated from `.cforge.toml` | No — edit `.cforge.toml` instead |
| `.cforge.toml` | `cforge config set` | Yes |
| `langs.txt` | `cforge new`, `cforge lang add/remove` | Yes |
| `libs.txt` | `cforge add` / `cforge remove` | Yes — the fallback when a library name does not resolve |
| `toolchain.txt` | `cforge toolchain use` | Yes |
| `deps.lock` | `cforge add` | Yes, but `deps sync` maintains it |
| `build/` | `cforge build` | Disposable — `cforge clean --all` removes it |

Commit `CMakeLists.txt`, `langs.txt`, `libs.txt`, `deps.lock`, and
`.cforge.toml`. Ignore `build/`.

---

## Templates

All five templates exist for C, C++, Objective-C, and Objective-C++, each
implemented in that language's own idioms rather than one language's template
with a different file extension.

> **The Objective-C templates require macOS**, as does Objective-C support
> generally. They target Apple's clang and Foundation. Rust has its own equivalents through
`new-rust-app`.

| Template | Contents |
|---|---|
| `cli` | A command-dispatcher app with `greet` and `version` subcommands, split across a main file and a commands module |
| `lib` | A compiled library: a public header in `include/`, sources in a directory with a `.cforge_lib` marker. `--lib-type static` (default) or `shared` |
| `header-lib` | A header-only library plus a small example program that uses it |
| `test` | An app plus a `_test` executable registered with CTest, so `cforge test` runs it |
| `server` | A minimal cross-platform TCP echo server |

```sh
cforge new-cpp-app myapp --template cli
cforge new mylib --lang c --template lib --lib-type shared
```

---

## Rust FFI

`cforge new <name> --ffi rust` scaffolds a C ABI boundary so a Rust program can
call into your C++ code:

- `include/<name>_ffi.h` — the C ABI header
- `CPP/<name>_ffi.cpp` — the implementation
- `bindings/` — a Rust crate that links against the built library

C++ is enabled automatically, since the implementation is a `.cpp` file. Build
the C++ side with `cforge build`, then build the Rust side with `cargo` inside
`bindings/`.

---

## Recipes

**Set up a project someone else created**

```sh
git clone <repo> && cd <repo>
cforge deps sync     # install every library in libs.txt
cforge build
```

**Release build with parallel jobs**

```sh
cforge build --profile release --jobs 8
```

**Wire up your editor's language server**

```sh
cforge compdb        # writes compile_commands.json
```

**Add a test to an existing project**

Create a directory whose name ends in `_test` under a source directory, put a
program with a `main()` in it, then:

```sh
cforge build && cforge test
```

**Use a specific compiler for one project**

```sh
cforge toolchain install cpp --compiler gcc --version 14
cforge toolchain use cpp g++-14
cforge clean --all && cforge build
```

**See what a command would do without doing it**

```sh
cforge new myapp --lang cpp --dry-run
```

---

## Troubleshooting

Run `cforge doctor` first — it checks compilers, CMake, and pinned standards
and reports what is wrong. Add `-v` to any command for detail, or `-vv` to
echo every command cforge runs underneath.

| Symptom | Cause and fix |
|---|---|
| `cforge: command not found` after installing | The PATH change has not reached your shell. Open a new terminal window |
| `No rule to make target '<name>'` | The target was never registered — the file may be in the wrong directory, or its language is not in `langs.txt`. Check `cforge lang list` |
| `'foo' is ambiguous` | `C/foo.c` and `CPP/foo.cpp` both exist. Build `foo.c` or `foo.cpp` explicitly |
| A build cannot find a library you added | The package manager name and the pkg-config/vcpkg module name differ. Find the real name with `cforge search <name>`, then fix `libs.txt` |
| Stale or nonsensical build errors | `cforge clean --all`, then `cforge build` |
| Wrong compiler being used | `cforge toolchain list`, then `cforge toolchain use <lang> <compiler>` |
| A pinned standard is rejected | `cforge std list` shows what your compiler actually supports; `cforge std set <key> latest` |
| Editor sees no include paths | `cforge compdb` |
| `obj_c is not supported on this platform` | Objective-C and Objective-C++ require macOS. Use C or C++ instead |
| Objective-C fails to build on macOS | Install Apple's Command Line Tools: `cforge toolchain install obj_c` |
| `cforge test` finds no tests | Tests come from directories ending in `_test`. Check the directory name, then `cforge build` before `cforge test` |

**Library name mismatches** are the most common failure. The name you pass to
`cforge add` has to match both the package manager's name and its
pkg-config/vcpkg module name, and those differ across platforms — OpenSSL is
`libssl-dev` on apt, `openssl-devel` on dnf, and `openssl` on Homebrew,
pacman, and vcpkg. When automatic resolution fails, edit `libs.txt` directly.
