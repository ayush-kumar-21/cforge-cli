# cforge

`cforge` is a command-line tool that sets up, builds, and runs C, C++,
Objective-C, and Objective-C++ projects for you — without you having to
hand-write a `CMakeLists.txt` or remember CMake's command-line flags.

If you've used `cargo` (Rust) or `npm`/`uv` before, `cforge` aims to feel
similar: `cforge new myapp`, then `cforge build`, then `cforge run` — no
separate "configure" step, no memorizing `-B build -S . -DCMAKE_BUILD_TYPE=...`.

Under the hood it generates and drives a normal CMake project, so anything
that already works with CMake still works — `cforge` is a friendlier layer
on top, not a replacement build system.

**Supported languages**: C, C++, and (macOS only) Objective-C / Objective-C++
— Objective-C requires Apple's runtime, so it isn't available on Linux/Windows.

**Supported platforms**: macOS, Linux (any distro), and Windows — as native
binaries. No WSL, no MSYS2, no bash required on Windows.

## Install

Pick your platform and run one command in a terminal.

**macOS / Linux**:
```sh
curl -fsSL https://raw.githubusercontent.com/ayush-kumar-21/cforge-cli/main/install.sh | sh
```

**Windows** (open PowerShell, then run):
```powershell
irm https://raw.githubusercontent.com/ayush-kumar-21/cforge-cli/main/install.ps1 | iex
```

Both commands download a ready-to-run `cforge` binary (no Rust toolchain or
compiling required) and add it to your PATH.

After installing, **open a new terminal window** (so the PATH change takes
effect) and check it worked:
```sh
cforge help
```

If your platform/CPU combo has no prebuilt binary (e.g. Linux on ARM), clone
this repo and run `cargo build --release` instead — that does require Rust
installed.

## Quickstart

Create a new C++ project called `myapp`:
```sh
cforge new myapp --lang cpp
cd myapp
```

This creates a `myapp/` folder with a starter source file and a generated
`CMakeLists.txt` — you don't need to edit either to get going.

Build it:
```sh
cforge build
```

Run it:
```sh
cforge run myapp
```

That's the whole loop: edit source files, `cforge build`, `cforge run`.

Other commands worth knowing early on:
```sh
cforge add <library>     # e.g. `cforge add sqlite3` — installs and links a library
cforge doctor            # checks your compiler/toolchain setup and reports problems
cforge help               # full command list
```

## Update / Uninstall

Update to the latest version at any time:
```sh
cforge update
```

Remove `cforge` completely (does **not** touch any projects you've built
with it — only the tool itself):
```sh
cforge uninstall
```

If `cforge` itself is broken and `cforge uninstall` won't run, use the
uninstall script directly from a clone of this repo instead:
`./uninstall.sh` (macOS/Linux) or `.\uninstall.ps1` (Windows).

## Good to know

- **Objective-C / Objective-C++** (`obj_c`/`obj_cpp` languages) only work on
  macOS — it's an Apple technology. On Linux/Windows, `cforge` sticks to
  C/C++ and never asks CMake for an Objective-C compiler, so plain C/C++
  projects build fine everywhere.
- **Windows compiler**: cforge uses LLVM's `clang`/`clang++` (auto-installed
  via `winget`, along with `ninja` as the build backend) instead of MSVC —
  this avoids requiring a multi-GB Visual Studio install just to compile.
- **Adding libraries** (`cforge add <lib>`): on Windows this goes through
  **vcpkg** (set up automatically on first use); on macOS/Linux it uses your
  system package manager (brew/apt/dnf/pacman/zypper/apk) plus pkg-config.
- **Library name mismatches**: the name you pass to `cforge add` needs to
  match both the package manager's name and its pkg-config/vcpkg module
  name, and these sometimes differ (e.g. OpenSSL's dev headers are
  `libssl-dev` on apt, `openssl-devel` on dnf, plain `openssl` on
  brew/pacman/vcpkg). If a build can't find a library, check the actual
  package name for your platform and edit `libs.txt` in your project if
  needed.
- Anything else `cforge` needs (a package manager, a C/C++ compiler, cmake,
  ninja on Windows) installs itself automatically the first time a command
  needs it — you shouldn't need to install these by hand first.

## Why Rust

Not for speed — profiling shows that over 99% of cforge's wall-clock time is
spent in the subprocesses it runs (cmake, the compiler), not in cforge's own
logic, so a faster implementation language wouldn't be noticeable. The real
reason is native Windows support: a compiled binary works on Windows without
requiring bash (WSL/MSYS2/Git Bash), which a shell script fundamentally can't
do.
