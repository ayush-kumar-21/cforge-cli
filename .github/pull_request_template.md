## What

<!-- What changes, in one or two sentences. -->

## Why

<!-- The problem this solves. Link the issue if there is one: Closes #123 -->

## How to test

<!-- Commands to run, or steps to reproduce the before/after. -->

## Platforms verified

<!-- cforge shells out to per-platform toolchains; tick what you actually ran on. -->

- [ ] Linux
- [ ] macOS
- [ ] Windows

## Checklist

- [ ] `cargo clippy --all-targets -- -D warnings` passes
- [ ] `cargo test` passes
- [ ] README / `--help` text updated if behaviour changed
- [ ] No direct commits to `main` — this branches off `feat/dev`
