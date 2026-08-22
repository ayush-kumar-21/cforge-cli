# Contributing to cforge

Thanks for taking a look. This is a small project — issues and pull requests
are both welcome.

## Getting set up

```sh
git clone https://github.com/ayush-kumar-21/cforge-cli
cd cforge-cli
cargo build
```

You need Rust **1.85 or newer** (the `rust-version` floor in `Cargo.toml`,
set by the `zeroize` dependency). CMake and a C/C++ compiler are needed to
run the integration tests, since those build real projects.

## Before you open a pull request

Run what CI runs. All five must pass:

```sh
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo test --locked --test scaffold_and_build -- --ignored --test-threads=1
shellcheck -s sh --severity=warning install.sh && shellcheck --severity=warning uninstall.sh
```

The fourth command is the slow one (~40s): it scaffolds and builds a real
project for every language × template combination through the compiled
binary. It is `#[ignore]`d by default so `cargo test` stays fast, which means
it is also the one most easily forgotten. Run it if you touched anything
under `src/templates/` or `src/cmake.rs`.

CI additionally runs `cargo audit` and a `cargo check` pinned to the MSRV.

## Code style

`rustfmt.toml` settles formatting — `max_width = 120` and
`use_small_heuristics = "Max"`, matching how the code was already written.
Run `cargo fmt` and don't argue with it.

Beyond that, the codebase has one strong convention: **comments explain why,
not what.** Most non-obvious code here carries a note about the bug or
platform quirk that shaped it. If you work out why something has to be a
certain way, write it down — that context is the expensive part.

## Project layout

| Path | Contents |
|---|---|
| `src/main.rs` | Argument parsing, command dispatch, help text |
| `src/build.rs` | Target discovery, build/run/test, library linking |
| `src/cmake.rs` | `CMakeLists.txt` generation and standards handling |
| `src/platform.rs` | OS detection, package managers, process helpers |
| `src/templates/` | One module per language, all implementing the same five templates |
| `src/selfmgmt.rs` | `self-update` / `self-uninstall`, checksum verification |
| `src/sha256.rs` | SHA-256 for release verification (NIST vectors in-file) |
| `tests/` | Integration tests that scaffold and build real projects |

## Adding a template

Templates live in `src/templates/<lang>.rs`, one module per language, and all
five categories (`cli`, `lib`, `header-lib`, `test`, `server`) exist for each.
A template should be idiomatic for its language, not a translation of the C++
one. Add the matching case to `tests/scaffold_and_build.rs` — a template that
is not built by a test is a template that will quietly stop compiling.

## Reporting bugs

Include your OS, the output of `cforge --version --verbose`, and the output of
`cforge doctor`. If a build failed, run the command again with `-vv` so the
underlying cmake/compiler invocation is in the report.

**Security issues do not go in the issue tracker** — see
[SECURITY.md](SECURITY.md) for private reporting.

## Licensing

Contributions are dual-licensed under MIT OR Apache-2.0, matching the project.
By submitting a pull request you agree your work ships under those terms.
