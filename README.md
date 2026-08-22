# cforge

**A build tool for C, C++, and Objective-C projects that works like `cargo`.**

Create a project, build it, run it — three commands, no `CMakeLists.txt` to
write, no CMake flags to memorize:

```sh
cforge new-cpp-app myapp --template cli
cd myapp
cforge build
cforge run myapp
```

Under the hood, cforge generates and drives an ordinary CMake project. Nothing
is locked in: anything that already works with CMake still works, and the
generated `CMakeLists.txt` is a normal file you can read, edit, or take with
you. cforge is a friendlier layer on top of CMake, not a replacement for it.

---

## Contents

- [Requirements](#requirements)
- [Install](#install)
- [Your first project](#your-first-project)
- [Everyday commands](#everyday-commands)
- [Project templates](#project-templates)
- [Adding libraries](#adding-libraries)
- [Updating and uninstalling](#updating-and-uninstalling)
- [Troubleshooting](#troubleshooting)
- [Platform notes](#platform-notes)

---

## Requirements

**Nothing but a terminal.** cforge installs what it needs — a C/C++ compiler,
CMake, Ninja on Windows, a package manager — the first time a command actually
needs it. You do not have to set up a toolchain by hand first.

| Platform | Status |
|---|---|
| macOS (Apple Silicon & Intel) | Fully supported |
| Linux (any distro) | Fully supported |
| Windows 10/11 | Fully supported — native binary, no WSL, MSYS2, or bash needed |

**Languages**: C, C++, Objective-C, and Objective-C++. A new project is
single-language (C by default); use `--lang` to enable more. There is also a
`new-rust-app` shortcut that hands off to `cargo` — cforge does not build Rust
projects, it just gets you started the same way.

Objective-C and Objective-C++ are fully supported on macOS. On Linux they work
through GNUstep, with caveats: no Apple frameworks, and ARC requires
`--runtime libobjc2`. On Windows they are unavailable, since Objective-C needs
either Apple's runtime or GNUstep and neither exists there.

---

## Install

Pick your platform and run one command.

**macOS / Linux**

```sh
curl -fsSL https://raw.githubusercontent.com/ayush-kumar-21/cforge-cli/main/install.sh | sh
```

**Windows** — open PowerShell and run:

```powershell
irm https://raw.githubusercontent.com/ayush-kumar-21/cforge-cli/main/install.ps1 | iex
```

Both commands download a ready-to-run `cforge` binary and add it to your PATH.
No Rust toolchain and no compiling required.

**Then open a new terminal window** so the PATH change takes effect, and check
that it worked:

```sh
cforge --version
cforge --help      # the full command list
```

> **Building from source**: if your platform/CPU combination has no prebuilt
> binary (Linux on ARM, for example), clone this repository and run
> `cargo build --release`. That path does require Rust installed.

---

## Your first project

### 1. Create the project

```sh
cforge new-cpp-app myapp --template cli
cd myapp
```

This creates a `myapp/` directory containing a small working command-line
program, a generated `CMakeLists.txt`, and the source layout cforge expects.
You do not need to edit any build files to get going.

Use `new-c-app`, `new-objc-app`, or `new-objcpp-app` for the other languages.
Leave off `--template` and cforge shows an arrow-key menu of the available
starter templates.

### 2. Build it

```sh
cforge build
```

There is no separate configure step — `cforge build` configures CMake the first
time and reuses that configuration afterwards.

### 3. Run it

```sh
cforge run myapp
```

The target name is the name of your app. To pass arguments to your program
rather than to cforge, put them after `--`:

```sh
cforge run myapp -- greet world
```

That is the whole loop: **edit source, `cforge build`, `cforge run`.**

### Starting from an empty project instead

If you would rather write everything yourself, `cforge new` creates the project
structure with no starter code:

```sh
cforge new myapp              # single-language C project
cforge new myapp --lang cpp   # or C++
cd myapp
```

You get a `CMakeLists.txt` plus source directories — `C/`, `CPP/`, `Obj_C/`,
`Obj_CPP/` for the languages you enabled, and `include/` for headers. Drop a
file with a `main()` into the matching directory and it becomes a build target
automatically:

```sh
echo 'int main(){return 0;}' > C/hello.c
cforge build
cforge run hello              # target name = the file name, minus extension
```

A subdirectory of source files (`CPP/myapp/main.cpp`, `CPP/myapp/util.cpp`)
builds into one executable named after that directory, so multi-file programs
need no configuration either.

Already have a directory of source files? Run `cforge init` inside it to set up
cforge in place instead of creating a new project.

---

## Everyday commands

Run `cforge --help` for the full list, or `cforge <command> -h` for details on
any one command. The [manual](MANUAL.md) documents every command, option, and
project file in full.

| Command | What it does |
|---|---|
| `cforge build [target]` | Build everything, or just one target |
| `cforge run <target> [-- args]` | Build and run a target |
| `cforge test [filter]` | Build and run tests through CTest |
| `cforge clean [--all]` | Remove build artifacts (`--all` also wipes the CMake cache) |
| `cforge add <library>` | Install and link a library |
| `cforge info` | Show the project's languages, standards, and libraries |
| `cforge doctor` | Diagnose your compiler and toolchain setup |
| `cforge format` / `cforge lint` | Run clang-format / clang-tidy |
| `cforge install` / `cforge package` | CMake install step / CPack packaging |

Useful global options: `--profile debug|release|relwithdebinfo`, `--jobs <n>`,
`--verbose`, `--quiet`, `--dry-run`.

---

## Project templates

Every language implements the same five templates, each written in that
language's own idioms — not one language's template stamped out with a
different file extension.

| Template | What you get |
|---|---|
| `cli` | A small command-dispatcher app with `greet` and `version` subcommands |
| `lib` | A compiled library — `--lib-type static` (default) or `shared` |
| `header-lib` | A header-only library, no separate build step to link |
| `test` | An app plus a test executable wired into `cforge test` |
| `server` | A minimal TCP echo server |

Pick one by language-specific command:

```sh
cforge new-c-app myapp                    # arrow-key menu of templates
cforge new-cpp-app myapp --template cli   # or name one directly
cforge new-objc-app myapp                 # macOS, or Linux via GNUstep
cforge new-objcpp-app myapp
cforge new-rust-app myapp                 # hands off to cargo new
```

Or set the language explicitly, if you prefer one command:

```sh
cforge new myapp --lang cpp --template lib --lib-type shared
```

---

## Adding libraries

```sh
cforge add sqlite3
```

cforge installs the library through your platform's package manager and links
it into the build — Homebrew, apt, dnf, pacman, zypper, or apk plus pkg-config
on macOS and Linux; vcpkg on Windows, set up automatically the first time.

Related commands: `cforge list` (linked libraries), `cforge remove <lib>`,
`cforge search <name>`, `cforge deps show` (resolved package names),
`cforge deps sync` (reinstall from the pinned `deps.lock`).

> **When a library name does not resolve**: the name you pass has to match both
> the package manager's name and the pkg-config/vcpkg module name, and those
> sometimes differ. OpenSSL, for instance, is `libssl-dev` on apt,
> `openssl-devel` on dnf, and plain `openssl` on Homebrew, pacman, and vcpkg.
> If a build cannot find a library, look up the real package name for your
> platform and edit `libs.txt` in your project.

---

## Updating and uninstalling

```sh
cforge self-update       # update to the latest release
cforge self-uninstall    # remove cforge and its PATH entry
```

Uninstalling removes only the tool. Projects you built with it are untouched.

If cforge itself is broken and `self-uninstall` will not run, use the uninstall
script directly from a clone of this repository: `./uninstall.sh` on
macOS/Linux, `.\uninstall.ps1` on Windows.

---

## Troubleshooting

**Start with `cforge doctor`.** It checks your compilers, CMake, and pinned
language standards, and reports what is missing or misconfigured.

| Symptom | Try this |
|---|---|
| `cforge: command not found` after installing | Open a new terminal window so the PATH change takes effect |
| A build cannot find a library you added | Check the real package name for your platform, then fix `libs.txt` (see [Adding libraries](#adding-libraries)) |
| Stale or confusing build errors | `cforge clean --all`, then `cforge build` |
| Wrong compiler being used | `cforge toolchain list`, then `cforge toolchain use <lang> <compiler>` |
| Editor or language server sees no include paths | `cforge compdb` writes `compile_commands.json` |

Add `--verbose` (or `-vv` to echo every command cforge runs) to any command to
see what is happening underneath. The [manual](MANUAL.md#troubleshooting) has a
longer troubleshooting table.

---

## Platform notes

- **Windows uses LLVM `clang`/`clang++`**, installed automatically via `winget`
  along with `ninja` as the build backend, rather than MSVC. This avoids
  requiring a multi-gigabyte Visual Studio install just to compile.
- **Objective-C on Linux** goes through GNUstep. Apple frameworks are not
  available, and ARC needs `--runtime libobjc2`. On Windows, cforge never asks
  CMake for an Objective-C compiler at all, so plain C/C++ projects build
  cleanly everywhere.
- **Everything else installs itself.** A package manager, a C/C++ compiler,
  CMake, Ninja on Windows — cforge sets these up the first time a command needs
  them.

---

## Contributing

Bug reports and pull requests are welcome — see [CONTRIBUTING.md](CONTRIBUTING.md)
for setup, the checks CI runs, and how the codebase is laid out. Security
issues have a [private reporting process](SECURITY.md).

## License

Dual-licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.
