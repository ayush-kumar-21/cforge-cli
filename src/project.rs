use crate::cmake::generate_cmakelists;
use crate::config::Config;
use crate::platform::{self, os, Os};
use std::fs;
use std::path::Path;

const KNOWN_LANGS: &[&str] = &["c", "cpp", "obj_c", "obj_cpp"];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LangCapability {
    Supported,
    Unsupported,
}

/// c/cpp work everywhere; obj_c/obj_cpp are macOS-only. Panics on an
/// unknown language — callers are expected to have already matched against
/// `KNOWN_LANGS`.
///
/// There used to be a third tier for Linux, where Objective-C nominally
/// worked through GNUstep. In practice it did not: GNUstep on Linux
/// compiles with GCC's Objective-C frontend, which predates Objective-C
/// 2.0 and rejects `@autoreleasepool`, array subscripting, and dot-syntax
/// — so the templates cforge itself scaffolds could not be built. Pointing
/// CMake at clang instead fails differently, on GNUstep's headers being
/// built against GCC's libobjc. Rather than advertise a tier that does not
/// work, Objective-C is macOS-only and says so up front.
pub fn lang_capability(lang: &str) -> LangCapability {
    match lang {
        "c" | "cpp" => LangCapability::Supported,
        "obj_c" | "obj_cpp" => match os() {
            Os::Macos => LangCapability::Supported,
            _ => LangCapability::Unsupported,
        },
        other => unreachable!("lang_capability called with unknown language '{other}'"),
    }
}

/// True if this platform can build obj_c/obj_cpp at all — used to gate
/// source-file discovery and CMakeLists.txt generation, which want a
/// yes/no rather than a per-language answer.
pub fn objc_capable_platform() -> bool {
    os() == Os::Macos
}

/// Platform-supported languages, used as the default set when nothing is
/// pinned in langs.txt: every known language on macOS, just c/cpp
/// everywhere else.
pub fn all_langs() -> &'static [&'static str] {
    if objc_capable_platform() {
        &["c", "cpp", "obj_c", "obj_cpp"]
    } else {
        &["c", "cpp"]
    }
}

pub fn validate_lang(lang: &str) {
    if !KNOWN_LANGS.contains(&lang) {
        crate::usage_error(&format!("unknown language '{lang}' (expected: {})", KNOWN_LANGS.join("|")));
    }
    if lang_capability(lang) == LangCapability::Unsupported {
        eprintln!("Error: {lang} is not supported on this platform.");
        eprintln!("  Objective-C and Objective-C++ are Apple platform languages and");
        eprintln!("  require macOS.");
        eprintln!("  Supported here: c, cpp");
        std::process::exit(3);
    }
}

fn write_langs_file(langs: &[String]) {
    let content = langs.iter().map(|l| format!("{l}\n")).collect::<String>();
    platform::write_file(Path::new("langs.txt"), &content);
}

pub(crate) fn read_langs_file() -> Vec<String> {
    fs::read_to_string("langs.txt").map(|s| s.lines().map(|l| l.to_string()).collect()).unwrap_or_default()
}

pub fn lang_add(langs: &[String]) {
    let mut current = read_langs_file();
    // No langs.txt yet: seed with platform defaults *before* applying the
    // requested add, so `lang add <lang>` on a fresh project extends the
    // default set instead of replacing it with just that one language (a
    // real regression this fixes — it previously only seeded defaults when
    // called with no arguments at all).
    if current.is_empty() {
        current = all_langs().iter().map(|s| s.to_string()).collect();
    }
    for l in langs {
        validate_lang(l);
        if !current.contains(l) {
            current.push(l.clone());
        }
    }
    current.sort();
    current.dedup();
    write_langs_file(&current);
    platform::status(&format!("Configured project for: {}", current.join(" ")));
}

pub fn lang_remove(langs: &[String]) {
    if langs.is_empty() {
        crate::usage_error("lang remove requires at least one language");
    }
    let mut current = read_langs_file();
    if current.is_empty() {
        current = all_langs().iter().map(|s| s.to_string()).collect();
    }
    for lang in langs {
        validate_lang(lang);
        current.retain(|l| l != lang);
    }
    // An empty langs.txt means "no languages pinned", which every reader
    // (build.rs's enabled_langs, lang_list, info) expands to *all*
    // platform-supported languages. So removing the last one did the
    // opposite of what was asked: `cforge lang remove c` on a C-only
    // project left it building c, cpp, obj_c and obj_cpp. Refuse instead
    // — the same way `toolchain remove` refuses to strip the last
    // compiler a language needs.
    if current.is_empty() {
        eprintln!(
            "Error: that would remove every language from the project, which reads as 'no languages pinned' \
             and builds all of them ({}). Add another language first, or delete langs.txt to opt into that default.",
            all_langs().join(", ")
        );
        std::process::exit(1);
    }
    write_langs_file(&current);
    platform::status(&format!("Configured project for: {}", current.join(" ")));
}

pub fn lang_list() {
    let current = read_langs_file();
    if current.is_empty() {
        println!("(none pinned — building with all supported languages: {})", all_langs().join(" "));
    } else {
        for lang in current {
            println!("{lang}");
        }
    }
}

/// Resolves the language for `cforge generate`/`cforge new` when --lang was
/// omitted, by looking at what's already enabled in langs.txt. Only answers
/// when that's unambiguous — exactly one language enabled — since a project
/// is always exactly one language at creation; a project with several
/// languages enabled (grown that way deliberately via `lang add`, after
/// creation) or none at all (langs.txt missing/empty) both still require
/// --lang spelled out rather than being guessed at.
pub fn infer_single_lang() -> String {
    match read_langs_file().as_slice() {
        [only] => only.clone(),
        [] => crate::usage_error(
            "generate requires --lang <lang> (no languages enabled — run 'cforge lang add <lang>' first, or pass --lang)",
        ),
        langs => crate::usage_error(&format!(
            "generate requires --lang <lang> (multiple languages enabled: {} — pass --lang to pick one)",
            langs.join(", ")
        )),
    }
}

pub fn generate(name: &str, ext: &str) {
    if name.is_empty() {
        crate::usage_error("generate requires a name");
    }
    validate_lang(ext);
    let cfg = Config::load();
    let (dir, file_ext) = match ext {
        "c" => (cfg.src_dir("c"), "c"),
        "cpp" => (cfg.src_dir("cpp"), "cpp"),
        "obj_c" => (cfg.src_dir("obj_c"), "m"),
        "obj_cpp" => (cfg.src_dir("obj_cpp"), "mm"),
        _ => unreachable!("validate_lang already rejected anything else"),
    };
    let file = Path::new(dir).join(format!("{name}.{file_ext}"));
    if file.exists() {
        eprintln!("Error: {} already exists.", file.display());
        std::process::exit(1);
    }
    platform::create_dir_all(Path::new(dir)).unwrap_or_else(|e| {
        eprintln!("Error: could not create {dir}: {e}");
        std::process::exit(1);
    });
    platform::write_file(&file, "");
    platform::status(&format!("Created {}", file.display()));
}

fn sanitize_project_name(raw: &str) -> String {
    let s: String = raw.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' }).collect();
    if s.is_empty() {
        "project".to_string()
    } else {
        s
    }
}

/// The project name as `init()` derives it: the current directory's
/// basename, sanitized. `new_project` passes whatever string the user
/// typed after `cforge new` (which may be a path like `/tmp/foo` or
/// `../foo`) — callers that need the *actual* project identifier (e.g.
/// FFI scaffolding naming its crate/symbols) should use this instead of
/// that raw argument, since directory name and CLI argument can differ.
pub fn current_project_name() -> String {
    let cwd = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
    let dir_name = cwd.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "project".to_string());
    sanitize_project_name(&dir_name)
}

pub fn init() {
    let cmakelists = Path::new("CMakeLists.txt");
    if cmakelists.exists() {
        println!("CMakeLists.txt already exists; skipping.");
        return;
    }

    let project_name = current_project_name();

    platform::write_file(cmakelists, &generate_cmakelists(&project_name));
    crate::cmake::generate_cforge_config();

    let cfg = Config::load();
    // Only create directories for languages actually enabled for this
    // project (langs.txt if it's already been written, else the platform
    // default set) — not all 4 unconditionally. Otherwise every project on
    // Windows gets a dead source directory for obj_c/obj_cpp, languages
    // that can never be built there.
    let enabled = read_langs_file();
    let enabled: Vec<&str> =
        if enabled.is_empty() { all_langs().to_vec() } else { enabled.iter().map(|s| s.as_str()).collect() };
    let src_dirs = enabled.iter().map(|lang| cfg.src_dir(lang));
    let dirs = src_dirs.chain(std::iter::once(cfg.paths.build.as_str()));
    for dir in dirs {
        platform::create_dir_all(Path::new(dir)).unwrap_or_else(|e| {
            eprintln!("Error: could not create {dir}: {e}");
            std::process::exit(1);
        });
    }
    platform::status("Initialized CMakeLists.txt and project directories.");
}

/// `langs` is 0 or 1 elements — `main.rs`'s `--lang` parsing only ever
/// yields one language now, unlike `lang_add`'s `<lang>...` (used by
/// `lang add`, where "add to what's there" is the right semantics for a
/// project growing into more languages after creation). Empty means "no
/// --lang given", which means "just c" — a project is exactly one language,
/// and this is where that language is chosen at creation time.
fn resolve_new_langs(langs: &[String]) -> Vec<String> {
    let mut explicit: Vec<String> = if langs.is_empty() {
        vec!["c".to_string()]
    } else {
        for l in langs {
            validate_lang(l);
        }
        langs.to_vec()
    };
    explicit.sort();
    explicit.dedup();
    explicit
}

/// Standards are no longer set here — they always start at whatever
/// `init()`'s generated CMakeLists.txt defaults to (latest), and `std set`
/// is the only way to change them, matching the reference doc's `new`
/// command which takes no standards arguments at all.
pub fn new_project(name: &str, langs: &[String]) {
    if name.is_empty() {
        crate::usage_error("new requires a project name");
    }
    platform::create_dir_all(Path::new(name)).unwrap_or_else(|e| {
        eprintln!("Error: could not create directory '{name}': {e}");
        std::process::exit(1);
    });
    // Under --dry-run, create_dir_all above was a no-op, so `name` doesn't
    // exist to cd into — the rest of this function runs against the
    // current directory instead, which is why dry-run paths below print
    // unprefixed by `name/`.
    if !crate::flags::get().dry_run {
        std::env::set_current_dir(name).unwrap_or_else(|e| {
            eprintln!("Error: could not enter directory '{name}': {e}");
            std::process::exit(1);
        });
    }

    // Installs a compiler if there's none, and asks which to use if the
    // machine has several — so the project is ready to build on exit.
    crate::toolchain::ensure_compiler_selected();
    let explicit = resolve_new_langs(langs);
    write_langs_file(&explicit);
    platform::status(&format!("Configured project for: {}", explicit.join(" ")));
    // init() reads langs.txt (written above) to decide which source
    // directories to create.
    init();
}

pub fn info() {
    let langs = read_langs_file();
    let langs_display =
        if langs.is_empty() { format!("{} (default: all supported)", all_langs().join(" ")) } else { langs.join(" ") };
    println!("Languages: {langs_display}");

    let cmakelists = Path::new("CMakeLists.txt");
    if cmakelists.exists() {
        let std_row = |key: &str, var: &str| {
            let val = crate::cmake::get_cmake_var(cmakelists, var).unwrap_or_else(|| "(not set)".to_string());
            println!("  {key:<8} {val}");
        };
        println!("Standards:");
        std_row("c", "CMAKE_C_STANDARD");
        std_row("cxx", "CMAKE_CXX_STANDARD");
        if objc_capable_platform() {
            std_row("objc", "CMAKE_OBJC_STANDARD");
            std_row("objcpp", "CMAKE_OBJCXX_STANDARD");
        }
    } else {
        println!("Standards: (no CMakeLists.txt — run 'cforge init' first)");
    }

    let libs = fs::read_to_string("libs.txt")
        .map(|s| s.lines().filter(|l| !l.is_empty()).map(|l| l.to_string()).collect::<Vec<_>>())
        .unwrap_or_default();
    if libs.is_empty() {
        println!("Libraries: (none)");
    } else {
        println!("Libraries: {}", libs.join(" "));
    }
}

#[cfg(test)]
mod capability_tests {
    use super::*;

    /// Locks in the fix for the macOS-only regression: `all_langs()`'s
    /// obj_c/obj_cpp membership must always agree with
    /// `objc_capable_platform()`, on whatever host runs this test.
    #[test]
    fn all_langs_agrees_with_objc_capability() {
        assert_eq!(all_langs().contains(&"obj_c"), objc_capable_platform());
        assert_eq!(all_langs().contains(&"obj_cpp"), objc_capable_platform());
    }

    /// c/cpp are always Supported; obj_c/obj_cpp are Supported only on
    /// macOS.
    #[test]
    fn capability_tiers_are_internally_consistent() {
        assert_eq!(lang_capability("c"), LangCapability::Supported);
        assert_eq!(lang_capability("cpp"), LangCapability::Supported);
        assert_eq!(lang_capability("obj_c") == LangCapability::Unsupported, !objc_capable_platform());
        assert_eq!(lang_capability("obj_cpp") == LangCapability::Unsupported, !objc_capable_platform());
    }

    /// Pins the rule itself, not just internal agreement: the assertions
    /// above are all phrased in terms of `objc_capable_platform()`, so they
    /// would keep passing if it started answering something else entirely.
    /// This is the one that fails on Linux and Windows CI if Objective-C
    /// stops being macOS-only.
    #[test]
    fn objc_is_macos_only() {
        assert_eq!(objc_capable_platform(), cfg!(target_os = "macos"));

        let expected = if cfg!(target_os = "macos") { LangCapability::Supported } else { LangCapability::Unsupported };
        assert_eq!(lang_capability("obj_c"), expected);
        assert_eq!(lang_capability("obj_cpp"), expected);

        // c/cpp must stay unaffected by the Objective-C rule on every host.
        assert!(all_langs().contains(&"c"));
        assert!(all_langs().contains(&"cpp"));
    }

    /// `cforge new` with no --lang is single-language (just c), not the
    /// full platform-supported set — the regression this locks in.
    #[test]
    fn no_explicit_lang_defaults_to_c_only() {
        assert_eq!(resolve_new_langs(&[]), vec!["c".to_string()]);
    }

    /// The CLI's --lang parsing only ever produces 0 or 1 elements now (a
    /// project is always exactly one language), so this only exercises
    /// that one-element case — the shape resolve_new_langs actually sees
    /// in practice.
    #[test]
    fn explicit_lang_is_used_as_is() {
        assert_eq!(resolve_new_langs(&["cpp".to_string()]), vec!["cpp".to_string()]);
    }
}

#[cfg(test)]
mod infer_single_lang_tests {
    use super::*;

    /// The one case `infer_single_lang` actually resolves: exactly one
    /// language enabled — the common case now that a project is
    /// single-language by default. The two ambiguous cases (zero or
    /// several languages enabled) exit(2) via usage_error, which can't be
    /// asserted on directly in-process (same reason validate_lang's
    /// Unsupported/exit(3) branch has no direct test either).
    #[test]
    fn resolves_when_exactly_one_lang_is_enabled() {
        crate::platform::in_scratch_dir("infer_one", || {
            write_langs_file(&["cpp".to_string()]);
            assert_eq!(infer_single_lang(), "cpp");
        });
    }
}
