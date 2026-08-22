//! `cforge doctor`. Beyond presence checks, this also validates that the
//! installed compiler actually supports whatever standard the current
//! project has pinned via `cforge std set` — presence alone doesn't catch
//! an old gcc that can't build a project pinned to c23/c++23.
use crate::cmake::{get_cmake_var, latest_c_std, latest_cxx_std, latest_objc_std, latest_objcxx_std};
use crate::color::{green, red, yellow};
use crate::platform::{c_compiler, cxx_compiler, detect_pkg_manager, os, probe_std, run_capture, Os, PkgManager};
use crate::project::all_langs;
use std::path::Path;

fn pkg_manager_name(m: PkgManager) -> &'static str {
    match m {
        PkgManager::Brew => "brew",
        PkgManager::Apt => "apt",
        PkgManager::Dnf => "dnf",
        PkgManager::Yum => "yum",
        PkgManager::Pacman => "pacman",
        PkgManager::Zypper => "zypper",
        PkgManager::Apk => "apk",
        PkgManager::Winget => "winget",
        PkgManager::None => "none",
    }
}

/// If the project has pinned a standard higher than the compiler supports,
/// report that pin as the failure reason instead of a bare "ok".
fn std_pin_status(cmake_var: &str, probe_flag: fn(&str) -> bool) -> Option<String> {
    let cmakelists = Path::new("CMakeLists.txt");
    let pinned = get_cmake_var(cmakelists, cmake_var)?;
    if probe_flag(&pinned) {
        None
    } else {
        Some(pinned)
    }
}

pub fn run() {
    println!("package manager: {}", pkg_manager_name(detect_pkg_manager()));

    match run_capture("cmake", &["--version"]) {
        Some(v) => println!("cmake: {} ({})", green("ok"), v.lines().next().unwrap_or(&v)),
        None => println!("cmake: {}", red("missing")),
    }

    for lang in ["c", "cpp", "obj_c", "obj_cpp"] {
        if !all_langs().contains(&lang) {
            println!("{lang}: {}", yellow(&format!("unsupported on {:?}", os())));
            continue;
        }
        match lang {
            "obj_c" | "obj_cpp" => {
                // Only reachable on macOS: all_langs() excludes obj_c/obj_cpp
                // elsewhere, and the branch above already printed
                // "unsupported" for them.
                let ok = run_capture("xcode-select", &["-p"]).is_some();
                let runtime = "apple-clang";
                if ok {
                    let bad_pin = if lang == "obj_c" {
                        std_pin_status("CMAKE_OBJC_STANDARD", |v| {
                            probe_std(c_compiler(), &format!("-std=c{v}"), "objective-c")
                        })
                    } else {
                        std_pin_status("CMAKE_OBJCXX_STANDARD", |v| {
                            probe_std(cxx_compiler(), &format!("-std=c++{v}"), "objective-c++")
                        })
                    };
                    match bad_pin {
                        None => println!("{lang}: {} ({runtime})", green("ok")),
                        Some(pin) => println!(
                            "{lang}: {} (present, but the pinned standard {pin} isn't supported — run 'cforge std set {} <lower>')",
                            red("version mismatch"),
                            if lang == "obj_c" { "objc" } else { "objcpp" }
                        ),
                    }
                } else {
                    println!("{lang}: {} (run: cforge toolchain install {lang})", red("missing"));
                }
            }
            _ => {
                if crate::platform::command_exists(c_compiler()) {
                    let bad_pin = if lang == "c" {
                        std_pin_status("CMAKE_C_STANDARD", |v| probe_std(c_compiler(), &format!("-std=c{v}"), "c"))
                    } else {
                        std_pin_status("CMAKE_CXX_STANDARD", |v| {
                            probe_std(cxx_compiler(), &format!("-std=c++{v}"), "c++")
                        })
                    };
                    let version = run_capture(c_compiler(), &["--version"]).unwrap_or_default();
                    let first_line = version.lines().next().unwrap_or("");
                    match bad_pin {
                        None => println!("{lang}: {} ({first_line})", green("ok")),
                        Some(pin) => println!(
                            "{lang}: {} (compiler present, but pinned standard {pin} isn't supported — run 'cforge std set {lang} <lower>' or upgrade the compiler)",
                            red("version mismatch")
                        ),
                    }
                } else {
                    println!("{lang}: {} (run: cforge toolchain install {lang})", red("missing"));
                }
            }
        }
    }

    if os() == Os::Macos {
        if let Some(v) = run_capture("xcrun", &["--sdk", "macosx", "--show-sdk-version"]) {
            println!("sdk: MacOSX{v}.sdk");
        }
    }

    // Surfaces what `latest` would currently resolve to, so a user pinned
    // to an old standard can see there's room to move up.
    println!();
    println!(
        "latest available standards: c={} cxx={}",
        latest_c_std().unwrap_or_else(|| "none".to_string()),
        latest_cxx_std().unwrap_or_else(|| "none".to_string())
    );
    if os() == Os::Macos {
        println!(
            "                             objc={} objcpp={}",
            latest_objc_std().unwrap_or_else(|| "none".to_string()),
            latest_objcxx_std().unwrap_or_else(|| "none".to_string())
        );
    }

    // Libraries this project depends on (libs.txt) — each is re-verified
    // against pkg-config right now, since a library added on a teammate's
    // machine and committed via libs.txt doesn't mean it's installed here.
    let libs = std::fs::read_to_string("libs.txt")
        .map(|s| s.lines().filter(|l| !l.is_empty()).map(|l| l.to_string()).collect::<Vec<_>>())
        .unwrap_or_default();
    if !libs.is_empty() {
        println!();
        println!("project libraries (libs.txt):");
        for lib in libs {
            if crate::pkgmgr::verify_pkg_config(&lib) {
                println!("  {lib}: {}", green("ok"));
            } else {
                println!("  {lib}: {} (run: cforge add {lib})", red("missing"));
            }
        }
    }
}
