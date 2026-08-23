# Changelog

## Unreleased

**`cforge new <name>` prompts interactively when run bare.** With no
`--lang` and no `--template`, and run from a terminal, it now shows an
npm-create-style picker — pick a language, then a starter template, both
single-select arrow-key menus. This is the same two-step flow
`new-<lang>-app` already had, except the language step is now also a menu
instead of being fixed by which command you typed. Passing `--lang` and/or
`--template` explicitly still skips the corresponding prompt, and anywhere
without a terminal (a script, CI) falls back to the previous default
silently: single language `c`, no template.

**`cforge generate` can add more than an empty file.** `--template <name>`
reuses the same five templates `cforge new --template` has, scaffolding a
library (or app, or test target) into a project that already exists —
`cforge generate mylib --template lib`. `--lang` is now optional on both
forms: when the project has exactly one language enabled, cforge infers it
rather than requiring every call to spell it out.

**`cforge new --lang` takes exactly one language now, not a list.** The
interactive picker above only ever lets you choose one, so letting the flag
accept several (`--lang c cpp`) was a leftover from before a project was
single-language — a way to end up somewhere the picker can't send you. A
project is always exactly one language at creation; `cforge lang add` is
still there for growing into more afterward, deliberately, as a separate
step. `--lang` on `cforge generate` was already single-valued.

## 0.1.0 — 2026-08-22

First public release.

cforge sets up, builds, and runs C, C++, Objective-C, and Objective-C++
projects without you writing a `CMakeLists.txt` or remembering CMake's
flags. It generates and drives an ordinary CMake project underneath, so
anything that already works with CMake still works.

### Project setup

- **`cforge new <name>`** scaffolds a project: `CMakeLists.txt`, the enabled
  languages' source directories, and `build/`. Single-language (`c`) unless
  `--lang` says otherwise.
- **npm-create-style shortcuts** pick the language for you: `new-c-app`,
  `new-cpp-app`, `new-objc-app`, `new-objcpp-app`, and `new-rust-app` (which
  delegates to `cargo new` — cforge does not build Rust projects itself). An
  arrow-key menu offers a starter template unless `--template` is given.
- **`cforge init`** sets cforge up in a directory of existing sources.
- **Five templates per language**, each written in that language's own
  idioms rather than one language's template with a different file
  extension: `cli`, `lib` (`--lib-type static|shared`), `header-lib`, `test`
  (registered with CTest), and `server` (a TCP echo server).
- **`cforge new --ffi rust`** scaffolds a C ABI boundary plus a Rust
  `bindings/` crate that links against it.

### Building

- **No configure step.** `cforge build` configures on first use and reuses
  it; `cforge run`, `cforge test`, `cforge clean`, `cforge install`, and
  `cforge package` cover the rest of the loop.
- **Targets are discovered, not registered.** A source file becomes an
  executable; a directory of same-language sources becomes one executable
  linked from all of them; a `.cforge_lib` marker makes it a library; a
  `*_test` directory name registers it with CTest.
- **Toolchains install themselves.** A package manager, a C/C++ compiler,
  CMake, and Ninja on Windows are installed on demand the first time a
  command needs one. `cforge doctor` diagnoses what is missing.
- **Layout is configurable** through `.cforge.toml` (`cforge config set`),
  and honored consistently by target resolution, `format`, `lint`, `clean`,
  `install`, and `package`.

### Dependencies

- **`cforge add <library>`** installs and links a library through the
  platform's package manager — Homebrew, apt, dnf, pacman, zypper, or apk
  plus pkg-config on Unix, vcpkg on Windows. Candidate package names are
  tried in order rather than giving up on the first miss.
- **`deps.lock`** records the name each library resolved to per platform, so
  `cforge deps sync` reproduces an environment on a fresh checkout or in CI.

### Platforms

macOS, Linux, and Windows, as native binaries — no WSL, MSYS2, or bash
required on Windows, which is the reason cforge is a compiled binary rather
than the shell script it started as.

**Objective-C and Objective-C++ require macOS.** They are Apple platform
languages, and cforge rejects them up front elsewhere rather than
half-working: GNUstep on Linux compiles with GCC's Objective-C frontend,
which predates Objective-C 2.0 and cannot build the templates cforge
scaffolds. C and C++ build everywhere.

### Security

Release binaries ship a `<asset>.sha256` generated on the same runner that
built them. `install.sh`, `install.ps1`, and `cforge self-update` all verify
it and are **fail-closed**: a mismatched checksum, or one that cannot be
fetched at all, aborts and leaves any existing binary untouched. The SHA-256
implementation is std-only and checked against the NIST FIPS 180-4 vectors
in-tree. See [SECURITY.md](SECURITY.md).

### Known limitations

- `cforge config set build <dir>` does not locate artifacts on Windows: the
  MSVC generator is multi-config and writes to `<build>/<Config>/`.
- `cforge new --ffi rust` produces a crate whose tests fail on Windows with
  `STATUS_DLL_NOT_FOUND`, because the linked CMake library is a DLL and
  `build/` is on neither the executable's directory nor PATH.

The integration tests for both are scoped to the platforms where they pass,
each with a comment naming the defect.
