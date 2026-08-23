mod build;
mod cmake;
mod codetools;
mod color;
mod config;
mod deps_lock;
mod doctor;
mod ffi;
mod flags;
mod packaging;
mod pkgmgr;
mod platform;
mod project;
mod selfmgmt;
mod sha256;
mod templates;
mod toolchain;
mod version;

use platform::os;

pub fn usage() -> ! {
    println!("Usage: cforge <command> [subcommand] [args] [options]");
    println!();
    println!("PROJECT SETUP");
    let row = |cmd: &str, args: &str, desc: &str| println!("    {cmd:<22} {args:<28} {desc}");
    row(
        "new",
        "<name> [opts]",
        "Scaffold a new project — bare, picks a language+template interactively; run 'cforge new -h' for opts",
    );
    row("new-c-app", "<name> [opts]", "Scaffold a C project; picks a template from a menu unless --template is given");
    row("new-cpp-app", "<name> [opts]", "Same as new-c-app, for C++");
    row("new-objc-app", "<name> [opts]", "Same as new-c-app, for Objective-C (macOS only)");
    row("new-objcpp-app", "<name> [opts]", "Same as new-c-app, for Objective-C++ (macOS only)");
    row(
        "new-rust-app",
        "<name> [opts]",
        "Scaffold a Rust project via cargo (not a cforge/CMake project); templates or picks from a menu",
    );
    row("init", "", "Initialize cforge in the current directory");
    row("info", "", "Show project config: languages, standards, libraries");
    println!();
    println!("BUILD & RUN");
    row("build", "[target...]", "Build one or more targets (all if omitted)");
    row("run", "<target> [-- <args>...]", "Build and run a target; args after -- go to the program");
    row("test", "[filter]", "Build and run tests via CTest");
    row("clean", "[--all]", "Remove build artifacts (--all also wipes the CMake cache)");
    row("install", "[--prefix <path>]", "Run the CMake install step");
    row("package", "[--format <fmt>]", "Package the project via CPack (tgz|zip|dmg|deb|rpm)");
    println!();
    println!("TOOLCHAIN");
    row("toolchain list", "", "Show detected compilers and any project overrides");
    row("toolchain install", "<lang> [opts]", "Install a compiler/runtime for a language");
    row("toolchain use", "<lang> <compiler>", "Pin a language to a specific compiler for this project");
    row("toolchain default", "", "Reset toolchain overrides to system defaults");
    row("toolchain remove", "<compiler>", "Uninstall a compiler (refuses if still in use)");
    row("doctor", "", "Diagnose the environment (compilers, cmake, pinned standards)");
    println!();
    println!("LANGUAGE CONFIGURATION");
    row("lang add", "<lang>...", "Enable languages for the build");
    row("lang remove", "<lang>...", "Disable languages from the build");
    row("lang list", "", "Show currently enabled languages");
    println!();
    println!("PROJECT LAYOUT");
    row("config show", "", "Show .cforge.toml source directory configuration");
    row("config set", "<key> <path>", "Set a source directory path");
    row("config reset", "", "Reset all paths to defaults");
    println!();
    println!("STANDARDS");
    row("std set", "<std-key> <value>", "Set a single language standard");
    row("std set", "--all latest", "Pin all supported standards to latest");
    row("std list", "", "Show current standards and what 'latest' resolves to");
    println!();
    println!("DEPENDENCIES");
    row("add", "<library>...", "Install + link a library/header (auto-resolves the platform package name)");
    row("remove", "<library>...", "Unlink a library/header");
    row("list", "", "List linked libraries");
    row("search", "<name>", "Search the OS package manager by name");
    row("deps show", "", "Show each linked library's resolved package name (deps.lock)");
    row("deps sync", "", "Reinstall all linked libraries using deps.lock's pinned package names");
    println!();
    println!("CODE");
    row("generate", "<name> [opts]", "Create a file, or --template for a library/app; run 'cforge generate -h'");
    row("format", "[path...]", "Run clang-format over source files");
    row("lint", "[path...]", "Run clang-tidy over source files");
    row("compdb", "", "Write compile_commands.json for editor/tool integration");
    println!();
    println!("TOOL MANAGEMENT");
    row("self-update", "", "Rebuild/reinstall cforge from its source or latest release");
    row("self-uninstall", "", "Remove cforge and its PATH entry");
    println!();
    println!("GLOBAL OPTIONS");
    row("-h, --help", "", "Show this help; run '<command> -h' for that command's own help");
    row("-V, --version", "", "Print version and exit (--verbose adds cmake/compiler/platform info)");
    row("-v, --verbose", "", "Verbose output (-vv also echoes every command)");
    row("-q, --quiet", "", "Suppress cforge's own status/progress messages (the underlying build tool's own output, e.g. compiler errors during 'build', is unaffected)");
    row("--profile <name>", "", "debug | release | relwithdebinfo");
    row("--jobs <n>", "", "Parallel build/test jobs");
    row("--no-color", "", "Disable ANSI color (also respects NO_COLOR)");
    row("--dry-run", "", "Print actions (including local file writes) without executing them");
    println!();
    println!("lang = c|cpp|obj_c|obj_cpp (obj_c/obj_cpp are macOS-only; c/cpp build everywhere)");
    println!("std keys = c|cxx|objc|objcpp (same four languages, spelled to match CMAKE_<X>_STANDARD)");
    println!();
    println!("Run 'cforge new <name>' to scaffold a new project directory, or 'cforge init' to use the current directory as-is.");
    println!("Missing dependencies (package manager, C/C++ compiler, cmake) are installed automatically when needed.");
    println!("Detected platform: {:?}", os());
    println!();
    println!("Examples:");
    for line in [
        "cforge new myapp",
        "cforge new myapp --lang cpp",
        "cforge new mytool --template cli",
        "cforge new mylib --template lib --lib-type shared",
        "cforge new myheaders --template header-lib",
        "cforge new mytested --template test",
        "cforge new myserver --template server",
        "cforge new-c-app myapp",
        "cforge new-cpp-app myapp --template cli",
        "cforge new-rust-app myapp --lib",
        "cforge init",
        "cforge info",
        "cforge build",
        "cforge build binarySearch",
        "cforge run binarySearch -- --flag value",
        "cforge test",
        "cforge clean --all",
        "cforge toolchain install obj_c",
        "cforge toolchain install cpp --compiler gcc --version 14",
        "cforge doctor",
        "cforge std set --all latest",
        "cforge std set cxx 20",
        "cforge std list",
        "cforge lang add cpp",
        "cforge lang list",
        "cforge add openssl",
        "cforge remove openssl",
        "cforge search openssl",
        "cforge generate binarySearch --lang cpp",
        "cforge generate mylib --template lib",
        "cforge format",
        "cforge package --format zip",
        "cforge self-update",
        "cforge self-uninstall",
    ] {
        println!("  {line}");
    }
    std::process::exit(0);
}

pub fn usage_error(msg: &str) -> ! {
    eprintln!("{} {msg}", color::red_err("error:"));
    eprintln!("Try 'cforge --help' for more information.");
    std::process::exit(2);
}

/// Per-subcommand `-h`/`--help` text (the known gap the bash version left
/// open: it only ever printed one global help screen). `cmd`/`sub` come
/// straight from argv, so an unrecognized pair just falls back to the
/// global usage screen rather than erroring.
fn subcommand_help(cmd: &str, sub: Option<&str>) -> ! {
    let text: Option<&str> = match (cmd, sub) {
        ("new", _) => Some(
            "Usage: cforge new <name> [--lang <lang>] [--ffi rust] [--template <name>] [--lib-type <kind>]\n\n\
             Scaffolds a new project directory: creates <name>/, writes CMakeLists.txt,\n\
             creates the language's source directory plus build/, and enables it (default:\n\
             c — a project is always exactly one language).\n\n\
             With neither --lang nor --template given, and run from an interactive\n\
             terminal, prompts for a language then a template instead — npm-create-style,\n\
             the same two-step flow 'new-<lang>-app' uses except the language step is a\n\
             menu too. Both are single-select. Pass --lang explicitly to skip the prompts\n\
             (a script, CI, or a non-interactive shell gets the old default automatically:\n\
             c, no template).\n\n\
             Options:\n\
             \x20\x20--lang <lang>      Language to enable: c|cpp|obj_c|obj_cpp\n\
             \x20\x20--ffi rust         Scaffold a C ABI boundary (include/<name>_ffi.h,\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20  src/<name>_ffi.cpp) and a bindings/ Rust crate that\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20  links against it — for Rust projects calling into C/C++.\n\
             \x20\x20--template <name>  Scaffold a real starter program, natively implemented\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20  for whichever language applies (--lang, or c if --lang\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20  was omitted). One of:\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20    cli         a command-line app\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20    lib         a compiled library (see --lib-type)\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20    header-lib  a header-only library + example\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20    test        an app plus a CTest-registered test executable\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20    server      a minimal cross-platform TCP echo server\n\
             \x20\x20--lib-type <kind>  static (default) or shared — only used with\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20  --template lib.",
        ),
        ("new-c-app", _) | ("new-cpp-app", _) | ("new-objc-app", _) | ("new-objcpp-app", _) => Some(
            "Usage: cforge new-<c|cpp|objc|objcpp>-app <name> [--template <name>] [--lib-type <kind>]\n\n\
             npm-create-style shortcut for 'cforge new': the language is fixed by which\n\
             command you run (no --lang needed). If --template isn't given and this is\n\
             an interactive terminal, an arrow-key menu picks one (or 'none' for a bare\n\
             project) — same template list as 'cforge new -h'. obj_c/obj_cpp are\n\
             macOS-only and exit(3) elsewhere.",
        ),
        ("new-rust-app", _) => Some(
            "Usage: cforge new-rust-app <name> [--template <name>] | [cargo-new options...]\n\n\
             A plain Rust/Cargo project, not a cforge/CMake one — cforge hands off to\n\
             cargo entirely. With --template (or the arrow-key menu, if this is an\n\
             interactive terminal and no --template/extra args were given), drives\n\
             'cargo new' itself and scaffolds idiomatic Rust source on top: cli (clap),\n\
             lib, header-lib (#[inline] utility crate), test (#[cfg(test)]), server\n\
             (std::net echo server). With neither, extra arguments pass straight\n\
             through to a plain 'cargo new', e.g. 'cforge new-rust-app mylib --lib'.\n\
             Requires cargo on PATH (rustup.rs).",
        ),
        ("init", _) => Some(
            "Usage: cforge init\n\n\
             Initializes cforge in the current directory (writes CMakeLists.txt and\n\
             creates the source directories) without scaffolding a new one.",
        ),
        ("info", _) => Some("Usage: cforge info\n\nShows enabled languages, pinned standards, and linked libraries."),
        ("build", _) => Some(
            "Usage: cforge build [target...]\n\n\
             Configures and builds. With no targets, builds every source file found\n\
             across the enabled languages' directories (src/ by default for all of\n\
             them). A target name is resolved by checking src/<name>.c,\n\
             src/<name>.cpp, and (macOS only) src/<name>.m, src/<name>.mm. If a\n\
             project has more than one language enabled and two share a base name\n\
             (src/foo.c and src/foo.cpp both exist), 'foo' is ambiguous — give the\n\
             extension to pick one: 'cforge build foo.c' or 'cforge build foo.cpp'.",
        ),
        ("run", _) => Some(
            "Usage: cforge run <target> [-- <args>...]\n\n\
             Builds a single target, then runs it. Arguments after -- are forwarded\n\
             to the program; the program's exit code becomes cforge's exit code.",
        ),
        ("test", _) => Some("Usage: cforge test [filter]\n\nRuns 'ctest' against build/. filter maps to 'ctest -R <filter>'."),
        ("clean", _) => Some(
            "Usage: cforge clean [--all]\n\n\
             Default: removes build outputs but keeps the CMake cache (fast reconfigure).\n\
             --all: removes the build/ directory entirely.",
        ),
        ("install", _) => Some(
            "Usage: cforge install [--prefix <path>]\n\n\
             Runs the CMake install step (installs built executables to <prefix>/bin).\n\
             With no --prefix, CMake's own default applies (/usr/local on macOS and\n\
             Linux). Requires a prior 'cforge build'.",
        ),
        ("package", _) => Some(
            "Usage: cforge package [--format <fmt>]\n\n\
             Packages the project via CPack. --format: tgz|zip|dmg|deb|rpm (default tgz).\n\
             Requires a prior 'cforge build'.",
        ),
        ("doctor", _) => Some(
            "Usage: cforge doctor\n\n\
             Diagnoses the environment: package manager, cmake, and per-language compiler\n\
             presence. Also checks the current project's pinned standards (cforge std set)\n\
             against what the installed compiler actually supports.",
        ),
        ("toolchain", Some("install")) => Some(
            "Usage: cforge toolchain install <lang> [--compiler <name>] [--version <v>] [--wait]\n\n\
             Installs a compiler for <lang> (c|cpp|obj_c|obj_cpp).\n\
             --compiler <gcc|clang|mingw|msvc|apple-clang>  (c/cpp only)\n\
             --version <v>       Compiler version, where the package manager supports it\n\
             --wait              obj_c/obj_cpp: block until the CLT installer finishes\n\n\
             obj_c/obj_cpp are macOS-only and install via Apple's Command Line Tools.",
        ),
        ("toolchain", Some("use")) => Some(
            "Usage: cforge toolchain use <lang> <compiler>\n\n\
             Pins <lang> to <compiler> for this project: recorded in toolchain.txt and\n\
             passed to cmake as -DCMAKE_<LANG>_COMPILER on the next 'cforge build'.",
        ),
        ("toolchain", Some("remove")) => Some(
            "Usage: cforge toolchain remove <compiler>\n\n\
             Uninstalls <compiler> via the OS package manager. Refuses if toolchain.txt\n\
             still names it for any language (run 'toolchain use' or 'toolchain default' first).",
        ),
        ("toolchain", Some("list")) => Some("Usage: cforge toolchain list\n\nShows detected cc/c++ and any project overrides in toolchain.txt."),
        ("toolchain", Some("default")) => Some("Usage: cforge toolchain default\n\nResets toolchain.txt, returning to system-default compilers."),
        ("toolchain", _) => Some("Usage: cforge toolchain list|install|use|default|remove ...\n\nRun 'cforge toolchain <subcommand> -h' for details on a specific one."),
        ("lang", Some("add")) => Some(
            "Usage: cforge lang add <lang>...\n\n\
             Enables the given languages (c|cpp|obj_c|obj_cpp) for the build, in addition to\n\
             whatever's already enabled. 'cforge new' only ever starts a project at one\n\
             language, but nothing stops it growing into more over time — this is that\n\
             deliberate, explicit step; it never happens as a side effect of 'new' itself.",
        ),
        ("lang", Some("remove")) => Some("Usage: cforge lang remove <lang>...\n\nDisables the given languages from the build."),
        ("lang", _) => Some("Usage: cforge lang add|remove|list <lang>..."),
        ("std", Some("set")) => Some(
            "Usage: cforge std set <c|cxx|objc|objcpp> <value>\n       cforge std set --all latest\n\n\
             <value> is a standard year (e.g. 20) or 'latest'.",
        ),
        ("std", _) => Some("Usage: cforge std set <key> <value> | cforge std list"),
        ("add", _) => Some("Usage: cforge add <library>...\n\nInstalls and links libraries via the OS package manager / pkg-config."),
        ("remove", _) => Some("Usage: cforge remove <library>...\n\nUnlinks previously added libraries."),
        ("search", _) => Some("Usage: cforge search <name>\n\nSearches the OS package manager for <name>."),
        ("deps", Some("show")) => Some("Usage: cforge deps show\n\nShows each library in libs.txt alongside the exact package name it resolved to on this platform (from deps.lock)."),
        ("deps", Some("sync")) => Some(
            "Usage: cforge deps sync\n\n\
             Reinstalls every library in libs.txt. Prefers the package name pinned in\n\
             deps.lock for this platform (falls back to re-resolving if that pin no\n\
             longer verifies). Run this on a fresh checkout or in CI instead of\n\
             repeating 'cforge add' for every dependency.",
        ),
        ("deps", _) => Some("Usage: cforge deps show|sync\n\nRun 'cforge deps <subcommand> -h' for details on a specific one."),
        ("generate", _) => Some(
            "Usage: cforge generate <name> [--lang <lang>] [--template <name>] [--lib-type <kind>]\n\n\
             Adds to the current project. By default, a single empty source file. With\n\
             --template, the same five real starter templates 'cforge new --template' has —\n\
             this is how to add a library (or app, or test target) to a project that\n\
             already exists: 'cforge generate mylib --template lib'.\n\n\
             Options:\n\
             \x20\x20--lang <lang>       c|cpp|obj_c|obj_cpp. Inferred when the project has\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20 exactly one language enabled; required otherwise.\n\
             \x20\x20--template <name>   cli|lib|header-lib|test|server\n\
             \x20\x20--lib-type <kind>   static (default) or shared — only used with\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20 --template lib.",
        ),
        ("format", _) => Some("Usage: cforge format [path...]\n\nRuns clang-format -i over the given paths (default: all source dirs)."),
        ("lint", _) => Some("Usage: cforge lint [path...]\n\nRuns clang-tidy over the given paths (default: all source dirs)."),
        ("compdb", _) => Some("Usage: cforge compdb\n\nWrites compile_commands.json at the project root."),
        ("config", Some("show")) => Some("Usage: cforge config show\n\nShows current .cforge.toml settings (source directories, build dir)."),
        ("config", Some("set")) => Some("Usage: cforge config set <key> <path>\n\nSets a source directory path. Keys: c_src, cpp_src, obj_c_src, obj_cpp_src, headers, build."),
        ("config", Some("reset")) => Some("Usage: cforge config reset\n\nResets all paths in .cforge.toml to defaults (src/ for every language, include/ for headers, build/)."),
        ("config", _) => Some("Usage: cforge config show|set|reset ...\n\nRun 'cforge config <subcommand> -h' for details on a specific one."),
        _ => None,
    };
    match text {
        Some(t) => {
            println!("{t}");
            std::process::exit(0);
        }
        None => usage(),
    }
}

/// True if `args` requests help for the *subcommand itself* (as opposed to
/// the global flag scan in `flags::extract`, which already treats -h/--help
/// as position-independent — this just decides whether to show focused
/// subcommand help instead of the global screen).
fn wants_subcommand_help(args: &[String]) -> bool {
    args.iter().any(|a| a == "-h" || a == "--help")
}

/// Pulls a single-valued flag (e.g. "--ffi <target>" or "--lang <lang>")
/// out of `args`, returning (Some(value), remaining_args). `--lang` used to
/// need its own multi-value scanning loop back when a project could enable
/// several languages at once; a project is always exactly one language now,
/// so it takes exactly one value like every other flag here.
fn split_value_flag(args: &[String], flag: &str) -> (Option<String>, Vec<String>) {
    let mut value = None;
    let mut remaining = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == flag {
            i += 1;
            value = args.get(i).cloned();
        } else {
            remaining.push(args[i].clone());
        }
        i += 1;
    }
    (value, remaining)
}

fn split_on_double_dash(args: &[String]) -> (Vec<String>, Vec<String>) {
    match args.iter().position(|a| a == "--") {
        Some(idx) => (args[..idx].to_vec(), args[idx + 1..].to_vec()),
        None => (args.to_vec(), Vec::new()),
    }
}

/// Rust's std panics if a `println!`/`print!` write fails, and it fails
/// with EPIPE whenever stdout is piped into something that closes early
/// (`cforge doctor | head`, `cforge list | grep -m1 ...`). A CLI should
/// exit quietly there, not dump a panic backtrace.
fn install_broken_pipe_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let msg = info.payload().downcast_ref::<String>().cloned().unwrap_or_default();
        if msg.contains("Broken pipe") {
            std::process::exit(0);
        }
        default_hook(info);
    }));
}

fn main() {
    install_broken_pipe_hook();
    let raw: Vec<String> = std::env::args().skip(1).collect();
    // Split on a literal "--" FIRST, before flag extraction ever runs, so
    // anything after it (e.g. `cforge run foo -- --dry-run`) is opaque
    // passthrough for the target program, not a cforge flag to strip out.
    let (cli_args, program_args) = split_on_double_dash(&raw);

    // A bare top-level -h/--help (no command word) is the global screen;
    // once a command word is present, -h/--help means that command's help
    // and must NOT be swallowed by the global flag scan first.
    if cli_args.is_empty() || (cli_args.len() == 1 && wants_subcommand_help(&cli_args)) {
        let (parsed_flags, _) = flags::extract(&cli_args);
        flags::init(parsed_flags);
        usage();
    }

    let cmd_is_help_target = !cli_args[0].starts_with('-');
    if cmd_is_help_target && wants_subcommand_help(&cli_args[1..]) {
        let (parsed_flags, rest) = flags::extract(&cli_args);
        flags::init(parsed_flags);
        let sub = rest.get(1).map(|s| s.as_str());
        subcommand_help(&cli_args[0], sub);
    }

    let (parsed_flags, rest) = flags::extract(&cli_args);
    flags::init(parsed_flags.clone());

    if parsed_flags.help {
        usage();
    }
    if parsed_flags.version {
        version::print(parsed_flags.verbose > 0);
    }
    if rest.is_empty() {
        usage();
    }

    let cmd = rest[0].as_str();
    let args = &rest[1..];

    match cmd {
        "new" => {
            let (lang, remaining) = split_value_flag(args, "--lang");
            let (ffi, remaining) = split_value_flag(&remaining, "--ffi");
            let (template, remaining) = split_value_flag(&remaining, "--template");
            let (lib_type, remaining) = split_value_flag(&remaining, "--lib-type");
            if let Some(t) = &template {
                templates::validate_template(t);
            }
            let lib_type = lib_type.unwrap_or_else(|| "static".to_string());
            templates::validate_lib_type(&lib_type);

            // A truly bare `cforge new <name>` — no --lang, no --template —
            // goes interactive, npm-create-style: pick a language, then a
            // template, the same two-step flow `new-<lang>-app` uses except
            // the language step is a menu here too instead of being fixed by
            // which command you typed. Single-select for the language, same
            // as the template picker below — a project is one language.
            //
            // Any explicit --lang and/or --template opts out of the
            // corresponding prompt, so scripted/non-interactive use (and the
            // documented "empty project" flow, `cforge new foo --lang cpp`)
            // is unaffected. prompt_language() itself falls back to `None`
            // with nothing prompted at all when there's no tty to ask
            // (--quiet, --dry-run, or a non-interactive shell) or the user
            // cancels, in which case this behaves exactly as before: single
            // language "c", no template.
            let (lang, template, lib_type) = if lang.is_none() && template.is_none() {
                match templates::prompt_language() {
                    Some(picked) => {
                        let (t, lt) = templates::prompt_choice();
                        (Some(picked), t, lt)
                    }
                    None => (lang, template, lib_type),
                }
            } else {
                (lang, template, lib_type)
            };

            // Which language --template applies to: --lang (as typed, or
            // just picked above), or the single-language default (c) if it
            // was omitted entirely. Each of c/cpp/obj_c/obj_cpp has its own
            // native template implementation (see templates/mod.rs).
            let template_lang = lang.clone().unwrap_or_else(|| "c".to_string());
            let name = remaining.first().map(|s| s.as_str()).unwrap_or("");
            // A project is always one language: 0 or 1 elements, never more
            // — new_project()/resolve_new_langs() still take a slice since
            // that's also how new-<lang>-app below calls it, but --lang
            // itself can no longer name more than one.
            let langs: Vec<String> = lang.into_iter().collect();
            project::new_project(name, &langs);
            if let Some(target) = ffi {
                ffi::validate_ffi_target(&target);
                // new_project() already cd'd into the project directory,
                // so derive the identifier the same way init() named the
                // CMake project — not from `name`, which may be a path
                // (`cforge new /tmp/foo` → dir basename "foo", not "_tmp_foo").
                ffi::scaffold_rust_ffi(&project::current_project_name());
            }
            if let Some(t) = template {
                templates::scaffold(&t, &project::current_project_name(), &lib_type, &template_lang);
            }
        }
        // npm-create-style shortcuts: the language is picked by which
        // command you type (no --lang needed), then the template is picked
        // from an arrow-key menu (templates::prompt_choice) unless
        // --template was given explicitly. obj_c/obj_cpp still go through
        // project::new_project's validate_lang, so the existing macOS-only
        // rule still applies.
        "new-c-app" | "new-cpp-app" | "new-objc-app" | "new-objcpp-app" => {
            let lang = match cmd {
                "new-c-app" => "c",
                "new-cpp-app" => "cpp",
                "new-objc-app" => "obj_c",
                "new-objcpp-app" => "obj_cpp",
                _ => unreachable!(),
            };
            let (template, remaining) = split_value_flag(args, "--template");
            let (lib_type, remaining) = split_value_flag(&remaining, "--lib-type");
            let name = remaining.first().map(|s| s.as_str()).unwrap_or("");
            project::new_project(name, &[lang.to_string()]);
            let (template, lib_type) = if let Some(t) = template {
                templates::validate_template(&t);
                let lib_type = lib_type.unwrap_or_else(|| "static".to_string());
                templates::validate_lib_type(&lib_type);
                (Some(t), lib_type)
            } else {
                templates::prompt_choice()
            };
            if let Some(t) = template {
                templates::scaffold(&t, &project::current_project_name(), &lib_type, lang);
            }
        }
        // No CMake, no langs.txt — cforge doesn't build Rust projects, just
        // hands off to cargo for them (templates/rust.rs). --template (or
        // the arrow-key picker, same as the other new-*-app commands)
        // drives 'cargo new' itself and scaffolds idiomatic Rust source on
        // top; with neither given, any extra args pass straight through to
        // a plain 'cargo new' (e.g. 'cforge new-rust-app foo --lib').
        "new-rust-app" => {
            let (template, remaining) = split_value_flag(args, "--template");
            if remaining.is_empty() {
                usage_error("new-rust-app requires a project name");
            }
            let name = &remaining[0];
            let extra = &remaining[1..];
            if let Some(t) = &template {
                templates::rust::validate_template(t);
            }
            let picked = template.or_else(|| if extra.is_empty() { templates::rust::prompt_choice() } else { None });
            match picked {
                Some(t) => templates::rust::scaffold(name, &t),
                None => {
                    if !platform::command_exists("cargo") {
                        eprintln!("Error: cargo not found on PATH — install Rust from https://rustup.rs first.");
                        std::process::exit(1);
                    }
                    let status = std::process::Command::new("cargo").arg("new").arg(name).args(extra).status();
                    match status {
                        Ok(s) if s.success() => {}
                        Ok(s) => std::process::exit(s.code().unwrap_or(1)),
                        Err(e) => {
                            eprintln!("Error: could not run cargo: {e}");
                            std::process::exit(1);
                        }
                    }
                }
            }
        }
        "init" => project::init(),
        "info" => project::info(),
        "build" => {
            build::build(args);
        }
        "run" => {
            let target = args.first().map(|s| s.as_str()).unwrap_or_else(|| {
                usage_error("run requires a target name");
            });
            build::run(target, &program_args);
        }
        "test" => build::run_tests(args.first().map(|s| s.as_str())),
        "clean" => build::clean(args.iter().any(|a| a == "--all")),
        "install" => {
            let prefix = args.iter().position(|a| a == "--prefix").and_then(|i| args.get(i + 1)).map(|s| s.as_str());
            packaging::install(prefix);
        }
        "package" => {
            let format = args.iter().position(|a| a == "--format").and_then(|i| args.get(i + 1)).map(|s| s.as_str());
            packaging::package(format);
        }
        "toolchain" => match args.first().map(|s| s.as_str()) {
            Some("list") => toolchain::list(),
            Some("install") => toolchain_install(&args[1..]),
            Some("use") => {
                let lang =
                    args.get(1).map(|s| s.as_str()).unwrap_or_else(|| usage_error("toolchain use requires a language"));
                let compiler =
                    args.get(2).map(|s| s.as_str()).unwrap_or_else(|| usage_error("toolchain use requires a compiler"));
                toolchain::use_compiler(lang, compiler);
            }
            Some("default") => toolchain::default_toolchain(),
            Some("remove") => {
                let compiler = args
                    .get(1)
                    .map(|s| s.as_str())
                    .unwrap_or_else(|| usage_error("toolchain remove requires a compiler name"));
                toolchain::remove(compiler);
            }
            _ => usage_error("expected 'toolchain list|install|use|default|remove'"),
        },
        "doctor" => doctor::run(),
        "lang" => match args.first().map(|s| s.as_str()) {
            Some("add") => project::lang_add(&args[1..]),
            Some("remove") => project::lang_remove(&args[1..]),
            Some("list") => project::lang_list(),
            _ => usage_error("expected 'lang add', 'lang remove', or 'lang list'"),
        },
        "config" => match args.first().map(|s| s.as_str()) {
            Some("show") => config_show(),
            Some("set") => {
                let key = args.get(1).map(|s| s.as_str()).unwrap_or_else(|| usage_error("config set requires a key"));
                let path = args.get(2).map(|s| s.as_str()).unwrap_or_else(|| usage_error("config set requires a path"));
                config_set(key, path);
            }
            Some("reset") => config_reset(),
            _ => usage_error("expected 'config show', 'config set', or 'config reset'"),
        },
        "std" => match args.first().map(|s| s.as_str()) {
            Some("set") => set_standard(&args[1..]),
            Some("list") => cmake::std_list(std::path::Path::new("CMakeLists.txt")),
            _ => usage_error("expected 'std set' or 'std list'"),
        },
        "add" => {
            if args.is_empty() {
                usage_error("add requires at least one library name");
            }
            for lib in args {
                build::add_library(lib);
            }
        }
        "remove" => {
            if args.is_empty() {
                usage_error("remove requires at least one library name");
            }
            for lib in args {
                build::remove_library(lib);
            }
        }
        "list" => build::list_libs(),
        "search" => {
            build::search(args.first().map(|s| s.as_str()).unwrap_or_else(|| usage_error("search requires a query")))
        }
        "deps" => match args.first().map(|s| s.as_str()) {
            Some("sync") => build::deps_sync(),
            Some("show") => build::deps_show(),
            _ => usage_error("expected 'deps sync' or 'deps show'"),
        },
        "generate" => {
            let (lang, remaining) = split_value_flag(args, "--lang");
            let (template, remaining) = split_value_flag(&remaining, "--template");
            let (lib_type, remaining) = split_value_flag(&remaining, "--lib-type");
            if let Some(t) = &template {
                templates::validate_template(t);
            }
            let lib_type = lib_type.unwrap_or_else(|| "static".to_string());
            templates::validate_lib_type(&lib_type);
            let name = remaining.first().map(|s| s.as_str()).unwrap_or("");
            if name.is_empty() {
                // project::generate() already checks this on the no-template
                // path, but templates::scaffold() (the --template path) has
                // no such guard of its own — it would happily write into
                // "<src_dir>//..." otherwise. Check up front so both paths
                // get the same error.
                usage_error("generate requires a name");
            }
            // --lang is optional now: with exactly one language enabled —
            // the common case, since `cforge new` defaults to one — there's
            // nothing to disambiguate, so infer it instead of making every
            // call spell out --lang for a project that only has one choice.
            let lang = lang.unwrap_or_else(project::infer_single_lang);
            match template {
                // Full template into the current project, by name — the
                // "create a library named X" path: 'cforge generate mylib
                // --template lib' reuses the exact scaffolding 'cforge new
                // --template lib' does, just against a project that already
                // exists instead of a fresh one.
                Some(t) => templates::scaffold(&t, name, &lib_type, &lang),
                // Default: a single empty source file, as before.
                None => project::generate(name, &lang),
            }
        }
        "format" => codetools::format(args),
        "lint" => codetools::lint(args),
        "compdb" => codetools::compdb(),
        "self-update" => selfmgmt::update_self(),
        "self-uninstall" => selfmgmt::uninstall_self(),
        other => usage_error(&format!("unknown command '{other}' (see --help)")),
    }
}

fn toolchain_install(args: &[String]) {
    let lang = args.first().map(|s| s.as_str()).unwrap_or_else(|| usage_error("toolchain install requires a language"));
    let mut compiler = None;
    let mut version = None;
    let mut wait = false;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--compiler" => {
                i += 1;
                compiler = args.get(i).map(|s| s.as_str());
            }
            "--version" => {
                i += 1;
                version = args.get(i).map(|s| s.as_str());
            }
            "--wait" => wait = true,
            _ => {}
        }
        i += 1;
    }
    toolchain::install(lang, compiler, version, wait);
}

fn set_standard(args: &[String]) {
    use cmake::{set_c_std, set_cxx_std, set_objc_std, set_objcxx_std};
    use platform::{ensure_compiler, os as get_os, Os};
    use std::path::Path;

    ensure_compiler();
    let cmakelists = Path::new("CMakeLists.txt");
    match args.first().map(|s| s.as_str()) {
        Some("--all") => {
            let val = args.get(1).map(|s| s.as_str()).unwrap_or("latest");
            set_c_std(cmakelists, val);
            set_cxx_std(cmakelists, val);
            if get_os() == Os::Macos {
                set_objc_std(cmakelists, val);
                set_objcxx_std(cmakelists, val);
            }
        }
        Some(key) => {
            let val = args.get(1).map(|s| s.as_str()).unwrap_or_else(|| usage_error("std set requires a value"));
            match key {
                "c" => set_c_std(cmakelists, val),
                "cxx" => set_cxx_std(cmakelists, val),
                "objc" => set_objc_std(cmakelists, val),
                "objcpp" => set_objcxx_std(cmakelists, val),
                other => usage_error(&format!("unknown standard key '{other}' (expected c|cxx|objc|objcpp)")),
            }
        }
        None => usage_error("std set requires a standard key or --all"),
    }
}

fn config_show() {
    let cfg = config::Config::load();
    println!("Source Directories (.cforge.toml):");
    println!("  c:         {}", cfg.src_dir("c"));
    println!("  cpp:       {}", cfg.src_dir("cpp"));
    println!("  obj_c:     {}", cfg.src_dir("obj_c"));
    println!("  obj_cpp:   {}", cfg.src_dir("obj_cpp"));
    println!("  headers:   {}", cfg.paths.headers);
    println!("  build:     {}", cfg.paths.build);
}

fn config_set(key: &str, path: &str) {
    let mut cfg = config::Config::load();
    match key {
        "c_src" => cfg.paths.c_src = path.to_string(),
        "cpp_src" => cfg.paths.cpp_src = path.to_string(),
        "obj_c_src" => cfg.paths.obj_c_src = path.to_string(),
        "obj_cpp_src" => cfg.paths.obj_cpp_src = path.to_string(),
        "headers" => cfg.paths.headers = path.to_string(),
        "build" => cfg.paths.build = path.to_string(),
        other => usage_error(&format!(
            "unknown config key '{other}' (expected c_src|cpp_src|obj_c_src|obj_cpp_src|headers|build)"
        )),
    }
    cfg.save();
    cmake::generate_cforge_config();
    platform::status(&format!("Set {key} = {path}"));
}

fn config_reset() {
    let cfg = config::Config::default();
    cfg.save();
    cmake::generate_cforge_config();
    platform::status("Reset all paths to defaults");
}
