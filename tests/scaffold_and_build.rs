//! Integration tests: actually scaffold *and build* a real project for
//! every language/template combination, via the compiled `cforge` binary
//! — coverage the unit tests in `src/` don't provide, since none of them
//! invoke cmake or a real compiler. This is exactly the gap that let 25
//! hand-verified-once template combinations ship with zero regression
//! protection: change a template string, break the generated code, and
//! `cargo test` would still go green without these.
//!
//! Slow (each test configures + builds a real CMake/cargo project) and
//! needs a working toolchain (cmake, a C/C++ compiler, cargo for the Rust
//! templates), so these are `#[ignore]`d by default — run explicitly with
//! `cargo test -- --ignored --test-threads=1` (ci.yml does this as a
//! separate step from the fast unit-test run).
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn cforge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_cforge"))
}

/// Fresh scratch directory for one test, named after it so parallel runs
/// (and repeat local runs) never collide with each other or with anything
/// left over from a previous run.
fn scratch_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("cforge_integration_{label}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// `stdin(null)` makes this deterministic regardless of whether the test
/// itself runs under an interactive terminal — cforge's arrow-key/numbered
/// prompts all gate on `is_terminal()`, and a null stdin reads as "not a
/// terminal" the same way a CI runner's would.
fn run(bin: &Path, cwd: &Path, args: &[&str]) -> Output {
    Command::new(bin).args(args).current_dir(cwd).stdin(Stdio::null()).output().expect("failed to launch process")
}

fn assert_ok(out: &Output, what: &str) {
    assert!(
        out.status.success(),
        "{what} failed (exit {:?}):\nstdout:\n{}\nstderr:\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Scaffolds `template` via `new-<lang>-app` and runs `cforge build` on
/// the result — the C/C++/Obj-C/Obj-C++ path (a real CMake build).
fn scaffold_and_build(new_cmd: &str, template: &str) {
    let cforge = cforge_bin();
    let base = scratch_dir(&format!("{new_cmd}_{template}"));
    let out = run(&cforge, &base, &[new_cmd, "proj", "--template", template]);
    assert_ok(&out, &format!("'{new_cmd} proj --template {template}'"));

    let project_dir = base.join("proj");
    let out = run(&cforge, &project_dir, &["build"]);
    assert_ok(&out, &format!("'cforge build' for {new_cmd}/{template}"));

    std::fs::remove_dir_all(&base).ok();
}

/// Same, but for `new-rust-app` — builds with plain `cargo build` instead
/// of `cforge build`, since cforge hands Rust projects to cargo entirely.
fn scaffold_and_cargo_build(template: &str) {
    let cforge = cforge_bin();
    let base = scratch_dir(&format!("rust_{template}"));
    let out = run(&cforge, &base, &["new-rust-app", "proj", "--template", template]);
    assert_ok(&out, &format!("'new-rust-app proj --template {template}'"));

    let project_dir = base.join("proj");
    let out = Command::new("cargo")
        .arg("build")
        .current_dir(&project_dir)
        .stdin(Stdio::null())
        .output()
        .expect("failed to launch cargo");
    assert_ok(&out, &format!("'cargo build' for rust/{template}"));

    std::fs::remove_dir_all(&base).ok();
}

macro_rules! c_family_tests {
    // The optional meta goes on the module, not on each fn. Gating the
    // functions individually left `use super::scaffold_and_build` behind
    // with nothing referencing it, which is an unused import -- and CI
    // builds with -D warnings, so the Windows job failed on a lint in a
    // test that was deliberately compiled out there.
    ($lang_mod:ident, $new_cmd:literal $(, #[$meta:meta])?) => {
        $(#[$meta])?
        mod $lang_mod {
            use super::scaffold_and_build;

            #[test]
            #[ignore = "builds a real project via cmake + a compiler"]
            fn cli() { scaffold_and_build($new_cmd, "cli"); }

            #[test]
            #[ignore = "builds a real project via cmake + a compiler"]
            fn lib() { scaffold_and_build($new_cmd, "lib"); }

            #[test]
            #[ignore = "builds a real project via cmake + a compiler"]
            fn header_lib() { scaffold_and_build($new_cmd, "header-lib"); }

            #[test]
            #[ignore = "builds a real project via cmake + a compiler"]
            fn test_template() { scaffold_and_build($new_cmd, "test"); }

            #[test]
            #[ignore = "builds a real project via cmake + a compiler"]
            fn server() { scaffold_and_build($new_cmd, "server"); }
        }
    };
}

c_family_tests!(c, "new-c-app");
c_family_tests!(cpp, "new-cpp-app");
// macOS only, for two different reasons.
//
// Windows has no viable Objective-C toolchain at all (see project.rs's
// LangCapability); `new-objc-app`/`new-objcpp-app` exit(3) there by design.
//
// Linux is a known gap rather than a design decision. The templates use
// Objective-C 2.0 (@autoreleasepool), which GCC's Objective-C frontend does
// not implement, and Debian's GNUstep is built against GCC's libobjc, so
// pointing CMake at clang instead fails differently: clang cannot find
// <objc/objc.h>. Making this tier work means GNUstep with libobjc2 and the
// matching -fobjc-runtime flags, which is its own piece of work, not a
// tweak. Building Objective-C *by hand* on Linux still works, as it always
// did; it is the scaffolded templates that need a 2.0 frontend.
c_family_tests!(objc, "new-objc-app", #[cfg(target_os = "macos")]);
c_family_tests!(objcpp, "new-objcpp-app", #[cfg(target_os = "macos")]);

mod rust {
    use super::scaffold_and_cargo_build;

    #[test]
    #[ignore = "builds a real project via cargo"]
    fn cli() {
        scaffold_and_cargo_build("cli");
    }

    #[test]
    #[ignore = "builds a real project via cargo"]
    fn lib() {
        scaffold_and_cargo_build("lib");
    }

    #[test]
    #[ignore = "builds a real project via cargo"]
    fn header_lib() {
        scaffold_and_cargo_build("header-lib");
    }

    #[test]
    #[ignore = "builds a real project via cargo"]
    fn test_template() {
        scaffold_and_cargo_build("test");
    }

    #[test]
    #[ignore = "builds a real project via cargo"]
    fn server() {
        scaffold_and_cargo_build("server");
    }
}

/// `.cforge.toml` lets a project use its own directory layout, and every
/// scaffolding/CMake path already honored it — but target *resolution*
/// hardcoded `C`/`CPP`/..., so a relocated source tree discovered no
/// buildable targets and `cforge build` exited 0 having built nothing.
/// Silent success is exactly what a test has to pin down: asserting the
/// command succeeded would have passed against the bug.
///
/// Not run on Windows: `cforge config set build <dir>` lands artifacts
/// where this assertion cannot find them there, because the MSVC generator
/// is multi-config and writes into <build>/<Config>/ rather than <build>/.
/// A real gap, tracked separately -- gating it here keeps the bug visible
/// in this comment instead of hidden behind a red build.
#[test]
#[cfg_attr(windows, ignore = "custom build dirs vs. the multi-config MSVC generator")]
#[ignore = "builds a real project via cmake + a compiler"]
fn custom_source_and_build_dirs_still_build() {
    let cforge = cforge_bin();
    let base = scratch_dir("custom_layout");
    assert_ok(&run(&cforge, &base, &["new-c-app", "proj", "--template", "cli"]), "scaffold");

    let project_dir = base.join("proj");
    std::fs::rename(project_dir.join("C"), project_dir.join("src")).unwrap();
    assert_ok(&run(&cforge, &project_dir, &["config", "set", "c_src", "src"]), "config set c_src");
    assert_ok(&run(&cforge, &project_dir, &["config", "set", "build", "out"]), "config set build");
    std::fs::remove_dir_all(project_dir.join("build")).ok();

    assert_ok(&run(&cforge, &project_dir, &["build"]), "'cforge build' with a custom layout");
    assert!(
        project_dir.join("out").join("proj").exists(),
        "'cforge build' reported success but produced no executable in the configured build dir"
    );
}

/// `cforge new --ffi rust` spans two build systems: cforge/CMake produces
/// the shared library, then cargo links against it via a generated
/// build.rs. Nothing else here exercises that handoff, and only running
/// the generated crate's own test proves the Rust side actually resolved
/// and called the C++ symbol at runtime.
///
/// Not run on Windows: the generated crate links the CMake library as a
/// DLL, and the test binary exits with STATUS_DLL_NOT_FOUND because
/// Windows resolves DLLs from the executable's directory and PATH, neither
/// of which contains build/. Also a real gap, also tracked separately.
#[test]
#[cfg_attr(windows, ignore = "DLL search path for the linked CMake library")]
#[ignore = "builds a real project via cmake + cargo"]
fn rust_ffi_bindings_link_against_the_cmake_library() {
    let cforge = cforge_bin();
    let base = scratch_dir("ffi_rust");
    assert_ok(&run(&cforge, &base, &["new", "mylib", "--lang", "cpp", "--ffi", "rust"]), "scaffold --ffi rust");

    let project_dir = base.join("mylib");
    assert_ok(&run(&cforge, &project_dir, &["build"]), "'cforge build' for the FFI library");

    let bindings = project_dir.join("bindings");
    let out = Command::new("cargo")
        .arg("test")
        .current_dir(&bindings)
        .stdin(Stdio::null())
        .output()
        .expect("failed to launch cargo");
    assert_ok(&out, "'cargo test' in bindings/");

    std::fs::remove_dir_all(&base).ok();
}

/// `--ffi rust` writes a C++ implementation file, so it has to enable cpp
/// itself: without `--lang`, a project defaults to C-only and the very
/// next `cforge build` died with "No rule to make target". Uses the bare
/// `new` (no --lang) deliberately — that is the broken combination.
#[test]
#[ignore = "builds a real project via cmake + cargo"]
fn ffi_without_an_explicit_lang_enables_cpp_and_builds() {
    let cforge = cforge_bin();
    let base = scratch_dir("ffi_no_lang");
    assert_ok(&run(&cforge, &base, &["new", "foo", "--ffi", "rust"]), "scaffold --ffi rust with no --lang");

    let project_dir = base.join("foo");
    assert_ok(&run(&cforge, &project_dir, &["build"]), "'cforge build' after --ffi rust with no --lang");

    std::fs::remove_dir_all(&base).ok();
}

/// Re-running scaffolding must never clobber a generated file the user is
/// expected to edit — the C ABI header above all, where an overwrite
/// silently deletes every declaration they added.
#[test]
#[ignore = "scaffolds a real project"]
fn rescaffolding_ffi_preserves_hand_edits() {
    let cforge = cforge_bin();
    let base = scratch_dir("ffi_rescaffold");
    assert_ok(&run(&cforge, &base, &["new", "mylib", "--lang", "cpp", "--ffi", "rust"]), "first scaffold");

    let header = base.join("mylib").join("include").join("mylib_ffi.h");
    let edited = std::fs::read_to_string(&header).unwrap() + "\n/* hand-added */\n";
    std::fs::write(&header, &edited).unwrap();

    assert_ok(&run(&cforge, &base, &["new", "mylib", "--lang", "cpp", "--ffi", "rust"]), "second scaffold");
    assert_eq!(std::fs::read_to_string(&header).unwrap(), edited, "re-scaffolding overwrote a hand-edited file");

    std::fs::remove_dir_all(&base).ok();
}
