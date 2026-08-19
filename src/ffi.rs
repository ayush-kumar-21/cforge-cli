//! `cforge new <name> --lang cpp --ffi rust` — scaffolds a C/C++ library
//! exposed through a stable C ABI, plus a Rust crate that links against it.
//! This targets the Rust+C/C++ FFI market: crypto, compression, and
//! perf-critical code where a Rust project needs to call into existing
//! C/C++, or a C/C++ library wants Rust bindings.
use crate::config::Config;
use crate::platform;
use std::path::Path;

/// Only "rust" is implemented; anything else is a usage error rather than
/// silently doing nothing, so a typo (`--ffi rsut`) fails loudly.
pub fn validate_ffi_target(target: &str) {
    if target != "rust" {
        crate::usage_error(&format!("unknown --ffi target '{target}' (expected: rust)"));
    }
}

/// Sanitizes a project name into a valid Rust crate/identifier name:
/// lowercase, underscores instead of hyphens/spaces, must start with a
/// letter or underscore. Cargo crate names allow hyphens, but the
/// generated `lib.rs` also uses the name as a symbol prefix, where
/// hyphens aren't valid — so this picks the stricter identifier rule for
/// both, rather than tracking two different sanitized forms.
fn rust_ident(project_name: &str) -> String {
    let mut out: String = project_name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
        .collect();
    if out.is_empty() || out.chars().next().unwrap().is_ascii_digit() {
        out = format!("_{out}");
    }
    out
}

/// Scaffolds the C ABI boundary (include/<name>_ffi.h + a starter CPP
/// implementation) and a `bindings/` Rust crate that links against it.
/// Called after `project::init()` / `lang_add()` have already set up the
/// normal cforge project, so this only adds the FFI-specific pieces.
pub fn scaffold_rust_ffi(project_name: &str) {
    let ident = rust_ident(project_name);
    let cfg = Config::load();

    scaffold_c_abi_header(&ident, &cfg);
    scaffold_cpp_impl(&ident, &cfg);
    scaffold_rust_crate(project_name, &ident);

    platform::status("Scaffolded Rust FFI bindings in bindings/ — see bindings/src/lib.rs");
    platform::status("Build the C/C++ side first ('cforge build'), then 'cd bindings && cargo build'.");
}

fn scaffold_c_abi_header(ident: &str, cfg: &Config) {
    let header_dir = Path::new(&cfg.paths.headers);
    platform::create_dir_all(header_dir).unwrap_or_else(|e| {
        eprintln!("Error: could not create {}: {e}", header_dir.display());
        std::process::exit(1);
    });

    let guard = format!("{}_FFI_H", ident.to_uppercase());
    let content = format!(
        "#ifndef {guard}\n\
         #define {guard}\n\
         \n\
         #ifdef __cplusplus\n\
         extern \"C\" {{\n\
         #endif\n\
         \n\
         /* Stable C ABI boundary — every function here is what the Rust\n\
          * bindings in bindings/src/lib.rs declare via `extern \"C\"`.\n\
          * Keep signatures C-compatible: no C++ classes, references, or\n\
          * exceptions across this boundary. */\n\
         \n\
         int {ident}_add(int a, int b);\n\
         \n\
         #ifdef __cplusplus\n\
         }}\n\
         #endif\n\
         \n\
         #endif /* {guard} */\n"
    );
    let path = header_dir.join(format!("{ident}_ffi.h"));
    platform::write_file(&path, &content);
}

fn scaffold_cpp_impl(ident: &str, cfg: &Config) {
    let cpp_dir = Path::new(&cfg.paths.cpp_src);
    platform::create_dir_all(cpp_dir).unwrap_or_else(|e| {
        eprintln!("Error: could not create {}: {e}", cpp_dir.display());
        std::process::exit(1);
    });

    let content = format!(
        "#include \"{ident}_ffi.h\"\n\
         \n\
         /* Example implementation behind the C ABI declared in\n\
          * include/{ident}_ffi.h. Replace with real logic; keep the\n\
          * function signatures matching the header exactly. */\n\
         \n\
         extern \"C\" int {ident}_add(int a, int b) {{\n\
         \x20\x20\x20\x20return a + b;\n\
         }}\n"
    );
    let path = cpp_dir.join(format!("{ident}_ffi.cpp"));
    if !path.exists() {
        platform::write_file(&path, &content);
    }
}

fn scaffold_rust_crate(project_name: &str, ident: &str) {
    let bindings_dir = Path::new("bindings");
    let src_dir = bindings_dir.join("src");
    platform::create_dir_all(&src_dir).unwrap_or_else(|e| {
        eprintln!("Error: could not create {}: {e}", src_dir.display());
        std::process::exit(1);
    });

    let cargo_toml = format!(
        "[package]\n\
         name = \"{project_name}-sys\"\n\
         version = \"0.1.0\"\n\
         edition = \"2021\"\n\
         build = \"build.rs\"\n\
         \n\
         [lib]\n\
         name = \"{ident}\"\n\
         \n\
         [dependencies]\n"
    );
    platform::write_file(&bindings_dir.join("Cargo.toml"), &cargo_toml);

    // build.rs links against the CMake build output. This assumes the
    // C/C++ side was already built via `cforge build` (build/ contains
    // lib{ident}.so/.dylib or {ident}.dll — see cmake.rs's ALL_FFI_LIB_TARGETS);
    // it doesn't invoke cmake itself, since driving a second build system
    // from within `cargo build` is exactly the kind of hidden coupling
    // that makes FFI setups fragile. The rerun-if lines are limited to
    // what actually affects linking, not the whole crate.
    //
    // The rpath line matters on macOS/Linux: without it, the built binary
    // knows how to *link* against the dylib (via -l) but not where to
    // *find* it at runtime (macOS defaults the install name to
    // @rpath/lib{ident}.dylib with no rpath entry pointing anywhere,
    // so the binary aborts on startup with "Library not loaded"). An
    // absolute rpath to build/ is used — computed from CARGO_MANIFEST_DIR
    // rather than a relative path — so it resolves regardless of the
    // cargo invocation's working directory. This is skipped on Windows,
    // which resolves DLLs via PATH/same-directory instead of rpath, and
    // where `-Wl,-rpath` isn't a link.exe flag.
    let build_rs = format!(
        "fn main() {{\n\
         \x20\x20\x20\x20// Assumes 'cforge build' has already produced build/ with\n\
         \x20\x20\x20\x20// lib{ident}.so / lib{ident}.dylib / {ident}.dll. Adjust the search\n\
         \x20\x20\x20\x20// path if your CMake output directory differs.\n\
         \x20\x20\x20\x20let manifest_dir = std::env::var(\"CARGO_MANIFEST_DIR\").unwrap();\n\
         \x20\x20\x20\x20let lib_dir = std::path::Path::new(&manifest_dir).join(\"..\").join(\"build\");\n\
         \x20\x20\x20\x20println!(\"cargo:rustc-link-search=native={{}}\", lib_dir.display());\n\
         \x20\x20\x20\x20println!(\"cargo:rustc-link-lib=dylib={ident}\");\n\
         \x20\x20\x20\x20if std::env::var(\"CARGO_CFG_TARGET_OS\").as_deref() != Ok(\"windows\") {{\n\
         \x20\x20\x20\x20\x20\x20\x20\x20println!(\"cargo:rustc-link-arg=-Wl,-rpath,{{}}\", lib_dir.display());\n\
         \x20\x20\x20\x20}}\n\
         \x20\x20\x20\x20println!(\"cargo:rerun-if-changed=../include/{ident}_ffi.h\");\n\
         \x20\x20\x20\x20println!(\"cargo:rerun-if-changed=../build\");\n\
         }}\n"
    );
    platform::write_file(&bindings_dir.join("build.rs"), &build_rs);

    let lib_rs = format!(
        "//! Raw FFI declarations for the C ABI in `include/{ident}_ffi.h`,\n\
         //! plus a safe wrapper around each function. Add new functions to\n\
         //! *both* the C header and the `extern \"C\"` block below — the\n\
         //! signatures must match exactly or this is undefined behavior.\n\
         \n\
         #[allow(non_snake_case)]\n\
         mod raw {{\n\
         \x20\x20\x20\x20extern \"C\" {{\n\
         \x20\x20\x20\x20\x20\x20\x20\x20pub fn {ident}_add(a: i32, b: i32) -> i32;\n\
         \x20\x20\x20\x20}}\n\
         }}\n\
         \n\
         /// Safe wrapper around `{ident}_add`. The underlying C function has\n\
         /// no invariants to uphold (plain integers, no pointers), so this\n\
         /// call is sound without extra checks — a function taking\n\
         /// pointers/lengths would need bounds validation here instead.\n\
         pub fn add(a: i32, b: i32) -> i32 {{\n\
         \x20\x20\x20\x20unsafe {{ raw::{ident}_add(a, b) }}\n\
         }}\n\
         \n\
         #[cfg(test)]\n\
         mod tests {{\n\
         \x20\x20\x20\x20use super::*;\n\
         \n\
         \x20\x20\x20\x20#[test]\n\
         \x20\x20\x20\x20fn add_matches_the_c_implementation() {{\n\
         \x20\x20\x20\x20\x20\x20\x20\x20assert_eq!(add(2, 3), 5);\n\
         \x20\x20\x20\x20}}\n\
         }}\n"
    );
    let lib_path = src_dir.join("lib.rs");
    if !lib_path.exists() {
        platform::write_file(&lib_path, &lib_rs);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_ident_sanitizes_hyphens_and_case() {
        assert_eq!(rust_ident("My-Cool-Lib"), "my_cool_lib");
    }

    #[test]
    fn rust_ident_prefixes_leading_digit() {
        assert_eq!(rust_ident("123lib"), "_123lib");
    }
}
