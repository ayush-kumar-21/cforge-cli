use crate::config::Config;
use crate::platform::{c_compiler, cxx_compiler, probe_std};
use std::fs;
use std::path::Path;

/// In-place replacement of a `set(VAR <value>)` line in CMakeLists.txt.
/// Rust does this with plain string ops instead of shelling out to `sed`,
/// which sidesteps the BSD-vs-GNU `sed -i` incompatibility entirely.
///
/// Appends the line when the variable isn't already present. Without that,
/// this was a silent no-op on any CMakeLists.txt lacking the variable —
/// e.g. a project scaffolded on Windows (whose header omits the OBJC/OBJCXX
/// block entirely) then opened on macOS: `cforge std set objc 17` printed
/// "Pinned CMAKE_OBJC_STANDARD=17" and changed nothing.
pub fn set_cmake_var(path: &Path, var: &str, val: &str) {
    let content = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("Error: could not read {}: {e}", path.display());
        std::process::exit(1);
    });
    crate::platform::write_file(path, &replace_cmake_var(&content, var, val));
}

/// Pure string half of `set_cmake_var`, so the replace-vs-append behaviour
/// is testable without touching disk.
fn replace_cmake_var(content: &str, var: &str, val: &str) -> String {
    let prefix = format!("set({var} ");
    let mut replaced = false;
    let mut out: Vec<String> = content
        .lines()
        .map(|line| {
            if line.trim_start().starts_with(&prefix) {
                replaced = true;
                format!("set({var} {val})")
            } else {
                line.to_string()
            }
        })
        .collect();
    if !replaced {
        out.push(format!("set({var} {val})"));
    }
    out.join("\n") + "\n"
}

/// Pure string parse so this is testable without touching disk.
pub fn parse_cmake_var(content: &str, var: &str) -> Option<String> {
    let prefix = format!("set({var} ");
    content.lines().find_map(|line| {
        let trimmed = line.trim_start();
        if trimmed.starts_with(&prefix) {
            trimmed.strip_prefix(&prefix)?.strip_suffix(')').map(|s| s.to_string())
        } else {
            None
        }
    })
}

pub fn get_cmake_var(path: &Path, var: &str) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    parse_cmake_var(&content, var)
}

pub fn latest_c_std() -> Option<String> {
    for s in ["23", "17", "11"] {
        if probe_std(c_compiler(), &format!("-std=c{s}"), "c") {
            return Some(s.to_string());
        }
    }
    None
}

pub fn latest_cxx_std() -> Option<String> {
    for s in ["26", "23", "20", "17", "14", "11"] {
        if probe_std(cxx_compiler(), &format!("-std=c++{s}"), "c++") {
            return Some(s.to_string());
        }
    }
    None
}

pub fn latest_objc_std() -> Option<String> {
    for s in ["23", "17", "11"] {
        if probe_std(c_compiler(), &format!("-std=c{s}"), "objective-c") {
            return Some(s.to_string());
        }
    }
    None
}

pub fn latest_objcxx_std() -> Option<String> {
    for s in ["26", "23", "20", "17", "14", "11"] {
        if probe_std(cxx_compiler(), &format!("-std=c++{s}"), "objective-c++") {
            return Some(s.to_string());
        }
    }
    None
}

fn resolve_std(val: &str, latest: fn() -> Option<String>, kind: &str, probe: impl Fn(&str) -> bool) -> String {
    if val == "latest" {
        latest().unwrap_or_else(|| {
            eprintln!("Error: no supported {kind} standard found.");
            std::process::exit(1);
        })
    } else {
        if !probe(val) {
            eprintln!("Error: {kind} standard '{val}' is not supported by the compiler.");
            std::process::exit(1);
        }
        val.to_string()
    }
}

pub fn set_c_std(cmake_file: &Path, val: &str) {
    let resolved = resolve_std(val, latest_c_std, "C", |v| probe_std(c_compiler(), &format!("-std=c{v}"), "c"));
    set_cmake_var(cmake_file, "CMAKE_C_STANDARD", &resolved);
    crate::platform::status(&format!("Pinned CMAKE_C_STANDARD={resolved} in CMakeLists.txt"));
}

pub fn set_cxx_std(cmake_file: &Path, val: &str) {
    let resolved =
        resolve_std(val, latest_cxx_std, "C++", |v| probe_std(cxx_compiler(), &format!("-std=c++{v}"), "c++"));
    set_cmake_var(cmake_file, "CMAKE_CXX_STANDARD", &resolved);
    crate::platform::status(&format!("Pinned CMAKE_CXX_STANDARD={resolved} in CMakeLists.txt"));
}

pub fn set_objc_std(cmake_file: &Path, val: &str) {
    if !crate::project::objc_capable_platform() {
        eprintln!("Error: Objective-C is not supported on this platform (no viable toolchain).");
        std::process::exit(3);
    }
    let resolved = resolve_std(val, latest_objc_std, "Objective-C", |v| {
        probe_std(c_compiler(), &format!("-std=c{v}"), "objective-c")
    });
    set_cmake_var(cmake_file, "CMAKE_OBJC_STANDARD", &resolved);
    crate::platform::status(&format!("Pinned CMAKE_OBJC_STANDARD={resolved} in CMakeLists.txt"));
}

pub fn set_objcxx_std(cmake_file: &Path, val: &str) {
    if !crate::project::objc_capable_platform() {
        eprintln!("Error: Objective-C++ is not supported on this platform (no viable toolchain).");
        std::process::exit(3);
    }
    let resolved = resolve_std(val, latest_objcxx_std, "Objective-C++", |v| {
        probe_std(cxx_compiler(), &format!("-std=c++{v}"), "objective-c++")
    });
    set_cmake_var(cmake_file, "CMAKE_OBJCXX_STANDARD", &resolved);
    crate::platform::status(&format!("Pinned CMAKE_OBJCXX_STANDARD={resolved} in CMakeLists.txt"));
}

pub fn std_list(cmake_file: &Path) {
    let row = |key: &str, cmake_var: &str, latest: Option<String>| {
        let current = get_cmake_var(cmake_file, cmake_var).unwrap_or_else(|| "(not set)".to_string());
        let latest = latest.unwrap_or_else(|| "(none supported)".to_string());
        println!("{key:<8} current={current:<12} latest={latest}");
    };
    row("c", "CMAKE_C_STANDARD", latest_c_std());
    row("cxx", "CMAKE_CXX_STANDARD", latest_cxx_std());
    if crate::project::objc_capable_platform() {
        row("objc", "CMAKE_OBJC_STANDARD", latest_objc_std());
        row("objcpp", "CMAKE_OBJCXX_STANDARD", latest_objcxx_std());
    }
}

const HEADER_COMMON: &str = "\
set(CMAKE_CXX_STANDARD 26)
set(CMAKE_CXX_STANDARD_REQUIRED False)

set(CMAKE_C_STANDARD 23)
set(CMAKE_C_STANDARD_REQUIRED False)
";

const HEADER_OBJC: &str = "
set(CMAKE_OBJCXX_STANDARD 26)
set(CMAKE_OBJCXX_STANDARD_REQUIRED False)

set(CMAKE_OBJC_STANDARD 23)
set(CMAKE_OBJC_STANDARD_REQUIRED False)
";

const BODY: &str = r#"
include(CTest)
enable_testing()

# Source directory paths — configured via .cforge.toml, injected into CMake
# via .cforge_config.cmake. This allows flexible project layouts (src/ instead
# of C/, etc.). If .cforge_config.cmake doesn't exist yet, use defaults.
if(EXISTS "${CMAKE_SOURCE_DIR}/.cforge_config.cmake")
    include("${CMAKE_SOURCE_DIR}/.cforge_config.cmake")
else()
    set(CFORGE_C_SRC "C")
    set(CFORGE_CPP_SRC "CPP")
    set(CFORGE_OBJ_C_SRC "Obj_C")
    set(CFORGE_OBJ_CPP_SRC "Obj_CPP")
    set(CFORGE_HEADERS "include")
endif()

# Makes `#include "name.h"` resolve for headers under the configured
# headers directory (e.g. the C ABI header from `cforge new --ffi rust`)
# without every source file needing a relative "../include/..." path.
if(EXISTS "${CMAKE_SOURCE_DIR}/${CFORGE_HEADERS}")
    include_directories("${CFORGE_HEADERS}")
endif()

# Languages selected via `cforge lang add` / `cforge lang remove` are
# tracked in langs.txt; all supported ones build by default when it's absent.
if(EXISTS "${CMAKE_SOURCE_DIR}/langs.txt")
    file(STRINGS "${CMAKE_SOURCE_DIR}/langs.txt" ENABLED_LANGS)
else()
    set(ENABLED_LANGS c cpp obj_c obj_cpp)
endif()

# Objective-C is enabled here rather than in project() above, so a project
# that does not use it never makes CMake look for a compiler that may not
# exist. Listing OBJC in project() failed configure outright on any machine
# without an Objective-C compiler -- including for projects containing no
# Objective-C at all, which is most of them.
#
# check_language probes instead of hard-failing, so a project whose
# langs.txt asks for obj_c on a non-Apple machine (a checkout of a macOS
# project, say) reports what is wrong and skips those sources, rather than
# dying in a CMake stack trace.
include(CheckLanguage)
if("obj_c" IN_LIST ENABLED_LANGS)
    check_language(OBJC)
    if(CMAKE_OBJC_COMPILER)
        enable_language(OBJC)
    else()
        message(WARNING "No Objective-C compiler found; skipping ${CFORGE_OBJ_C_SRC}/. Objective-C requires macOS.")
        list(REMOVE_ITEM ENABLED_LANGS "obj_c")
    endif()
endif()
if("obj_cpp" IN_LIST ENABLED_LANGS)
    check_language(OBJCXX)
    if(CMAKE_OBJCXX_COMPILER)
        enable_language(OBJCXX)
    else()
        message(WARNING "No Objective-C++ compiler found; skipping ${CFORGE_OBJ_CPP_SRC}/. Objective-C++ requires macOS.")
        list(REMOVE_ITEM ENABLED_LANGS "obj_cpp")
    endif()
endif()

if("c" IN_LIST ENABLED_LANGS)
    file(GLOB C_FILES "${CFORGE_C_SRC}/*.c")
endif()
if("cpp" IN_LIST ENABLED_LANGS)
    file(GLOB CPP_FILES "${CFORGE_CPP_SRC}/*.cpp")
    # Files matching *_ffi.cpp (from `cforge new --ffi rust`) are the C ABI
    # boundary a Rust crate in bindings/ links against — they belong in a
    # shared library, not their own executable like every other CPP file.
    file(GLOB FFI_CPP_FILES "${CFORGE_CPP_SRC}/*_ffi.cpp")
    if(FFI_CPP_FILES)
        list(REMOVE_ITEM CPP_FILES ${FFI_CPP_FILES})
    endif()
endif()
if("obj_c" IN_LIST ENABLED_LANGS)
    file(GLOB OBJC_FILES "${CFORGE_OBJ_C_SRC}/*.m")
endif()
if("obj_cpp" IN_LIST ENABLED_LANGS)
    file(GLOB OBJCPP_FILES "${CFORGE_OBJ_CPP_SRC}/*.mm")
endif()

# Libraries added via `cforge add <name>` are tracked in libs.txt and
# resolved through pkg-config.
if(EXISTS "${CMAKE_SOURCE_DIR}/libs.txt")
    file(STRINGS "${CMAKE_SOURCE_DIR}/libs.txt" EXTRA_LIBS)
endif()

set_property(DIRECTORY APPEND PROPERTY CMAKE_CONFIGURE_DEPENDS
    "${CMAKE_SOURCE_DIR}/langs.txt" "${CMAKE_SOURCE_DIR}/libs.txt")
if(EXTRA_LIBS)
    find_package(PkgConfig REQUIRED)
    foreach(lib ${EXTRA_LIBS})
        pkg_check_modules(${lib} REQUIRED IMPORTED_TARGET ${lib})
        list(APPEND EXTRA_LIB_TARGETS PkgConfig::${lib})
    endforeach()
endif()

# Shared by add_lang_executables (one file -> one executable) and
# add_lang_apps (one directory of files -> one executable/library) below.
function(link_extra_libs exec_name needs_foundation)
    if(WIN32)
        # Winsock, needed by the `--template server` starter (BSD-socket
        # calls under WSAStartup). Always present in the Windows SDK/MinGW,
        # so linking it into every target is harmless for the rest.
        target_link_libraries(${exec_name} ws2_32)
    endif()
    # Objective-C is macOS-only, so Foundation is Apple's Foundation --
    # there is no GNUstep fallback to pick between.
    if(needs_foundation)
        target_link_libraries(${exec_name} "-framework Foundation")
    endif()
    if(EXTRA_LIB_TARGETS)
        target_link_libraries(${exec_name} ${EXTRA_LIB_TARGETS})
    endif()
endfunction()

function(add_lang_executables files suffix needs_foundation)
    foreach(source_file ${files})
        get_filename_component(base_name ${source_file} NAME_WE)
        set(exec_name "${base_name}_${suffix}")
        add_executable(${exec_name} ${source_file})
        link_extra_libs(${exec_name} ${needs_foundation})
        list(APPEND ALL_EXEC_TARGETS ${exec_name})
    endforeach()
    set(ALL_EXEC_TARGETS ${ALL_EXEC_TARGETS} PARENT_SCOPE)
endfunction()

# A subdirectory directly under a language's source directory (e.g.
# CPP/mygame/) whose files all share that language's extension becomes ONE
# executable named after the directory, built from every matching file
# inside it — the multi-file counterpart to a standalone CPP/foo.cpp file,
# for programs that need more than one source file linked together (a
# game's main.cpp + Player.cpp + Renderer.cpp as one binary, not three
# separate, unlinked ones).
#
# Two directory conventions, both used by `cforge new --template`, change
# what gets built instead of an executable:
#  - a ".cforge_lib" marker file inside the directory (content: STATIC or
#    SHARED) makes it a library instead (`--template lib`);
#  - a directory name ending in "_test" is additionally registered with
#    CTest via add_test (`--template test`) — ctest is already enabled by
#    `enable_testing()` above.
function(add_lang_apps src_dir ext needs_foundation)
    # IS_DIRECTORY/EXISTS in if() only have well-defined behavior with an
    # absolute path — src_dir arrives as the relative "CPP"/"C"/etc., which
    # silently evaluated false and made this return immediately every time.
    set(abs_dir "${CMAKE_SOURCE_DIR}/${src_dir}")
    if(NOT IS_DIRECTORY "${abs_dir}")
        return()
    endif()
    file(GLOB app_entries RELATIVE "${abs_dir}" "${abs_dir}/*")
    foreach(app_name ${app_entries})
        if(IS_DIRECTORY "${abs_dir}/${app_name}")
            file(GLOB app_sources "${abs_dir}/${app_name}/*.${ext}")
            if(app_sources)
                if(EXISTS "${abs_dir}/${app_name}/.cforge_lib")
                    file(READ "${abs_dir}/${app_name}/.cforge_lib" lib_kind)
                    string(STRIP "${lib_kind}" lib_kind)
                    add_library(${app_name} ${lib_kind} ${app_sources})
                    link_extra_libs(${app_name} ${needs_foundation})
                    list(APPEND ALL_LIB_TARGETS ${app_name})
                else()
                    add_executable(${app_name} ${app_sources})
                    link_extra_libs(${app_name} ${needs_foundation})
                    if(${app_name} MATCHES "_test$")
                        add_test(NAME ${app_name} COMMAND ${app_name})
                    endif()
                    list(APPEND ALL_EXEC_TARGETS ${app_name})
                endif()
            endif()
        endif()
    endforeach()
    set(ALL_EXEC_TARGETS ${ALL_EXEC_TARGETS} PARENT_SCOPE)
    set(ALL_LIB_TARGETS ${ALL_LIB_TARGETS} PARENT_SCOPE)
endfunction()

add_lang_executables("${C_FILES}" c FALSE)
add_lang_executables("${CPP_FILES}" cpp FALSE)
add_lang_executables("${OBJC_FILES}" objc TRUE)
add_lang_executables("${OBJCPP_FILES}" objcpp TRUE)

if("c" IN_LIST ENABLED_LANGS)
    add_lang_apps("${CFORGE_C_SRC}" c FALSE)
endif()
if("cpp" IN_LIST ENABLED_LANGS)
    add_lang_apps("${CFORGE_CPP_SRC}" cpp FALSE)
endif()
if("obj_c" IN_LIST ENABLED_LANGS)
    add_lang_apps("${CFORGE_OBJ_C_SRC}" m TRUE)
endif()
if("obj_cpp" IN_LIST ENABLED_LANGS)
    add_lang_apps("${CFORGE_OBJ_CPP_SRC}" mm TRUE)
endif()

# *_ffi.cpp files (from `cforge new --ffi rust`) build as a SHARED library
# instead of an executable — that's what bindings/build.rs links the Rust
# crate against via `cargo:rustc-link-lib`. The "_ffi" suffix is stripped
# from the output name so the library is libNAME.so/.dylib/NAME.dll,
# matching the `[lib] name = "NAME"` the scaffolded Cargo.toml declares.
foreach(source_file ${FFI_CPP_FILES})
    get_filename_component(base_name ${source_file} NAME_WE)
    string(REGEX REPLACE "_ffi$" "" ffi_lib_name ${base_name})
    add_library(${ffi_lib_name} SHARED ${source_file})
    if(EXTRA_LIB_TARGETS)
        target_link_libraries(${ffi_lib_name} ${EXTRA_LIB_TARGETS})
    endif()
    list(APPEND ALL_FFI_LIB_TARGETS ${ffi_lib_name})
endforeach()

# Backs `cforge install`: every target this project produces installs to
# <prefix>/bin (default /usr/local/bin, override with --prefix).
if(ALL_EXEC_TARGETS)
    install(TARGETS ${ALL_EXEC_TARGETS} RUNTIME DESTINATION bin)
endif()
if(ALL_FFI_LIB_TARGETS)
    install(TARGETS ${ALL_FFI_LIB_TARGETS}
        RUNTIME DESTINATION bin
        LIBRARY DESTINATION lib
        ARCHIVE DESTINATION lib)
endif()
# `--template lib` targets (add_lang_apps' .cforge_lib marker), separate
# from the FFI shared libraries above since those are always SHARED.
if(ALL_LIB_TARGETS)
    install(TARGETS ${ALL_LIB_TARGETS}
        RUNTIME DESTINATION bin
        LIBRARY DESTINATION lib
        ARCHIVE DESTINATION lib)
endif()

set(CPACK_PROJECT_NAME ${PROJECT_NAME})
set(CPACK_PROJECT_VERSION ${PROJECT_VERSION})
include(CPack)
"#;

/// Generate `.cforge_config.cmake` with configured source paths.
/// This file is included by CMakeLists.txt and sets CFORGE_* variables
/// to allow flexible project structures (e.g., src/ instead of C/).
pub fn generate_cforge_config() {
    let cfg = Config::load();
    let content = format!(
        "# Auto-generated by cforge — do not edit directly. Modify .cforge.toml instead.\n\
         set(CFORGE_C_SRC \"{}\")\n\
         set(CFORGE_CPP_SRC \"{}\")\n\
         set(CFORGE_OBJ_C_SRC \"{}\")\n\
         set(CFORGE_OBJ_CPP_SRC \"{}\")\n\
         set(CFORGE_HEADERS \"{}\")\n",
        cfg.src_dir("c"),
        cfg.src_dir("cpp"),
        cfg.src_dir("obj_c"),
        cfg.src_dir("obj_cpp"),
        cfg.paths.headers,
    );
    crate::platform::write_file(Path::new(".cforge_config.cmake"), &content);
}

/// Builds the CMakeLists.txt content for a fresh project.
///
/// project() requests only C and CXX. OBJC/OBJCXX used to be listed here
/// whenever the *platform* could support them (macOS or Linux), which meant
/// CMake went hunting for an Objective-C compiler while configuring a
/// pure-C project. On macOS that is free — clang is one compiler — but on a
/// stock Linux box gcc cannot compile Objective-C without the gobjc
/// package, so `cforge build` died with "cannot execute cc1obj" on projects
/// containing no Objective-C at all.
///
/// They are enabled by `enable_language` in BODY instead, gated on
/// langs.txt. That is evaluated at configure time rather than baked in
/// here, so `cforge lang add obj_c` on an existing project now takes effect
/// without regenerating CMakeLists.txt.
pub fn generate_cmakelists(project_name: &str) -> String {
    let objc_ok = crate::project::objc_capable_platform();

    let mut out =
        format!("cmake_minimum_required(VERSION 3.16)\nproject({project_name} VERSION 0.1.0 LANGUAGES C CXX)\n\n");
    out.push_str(HEADER_COMMON);
    if objc_ok {
        out.push_str(HEADER_OBJC);
    }
    out.push_str(BODY);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_matching_var() {
        let content = "set(CMAKE_CXX_STANDARD 20)\nset(CMAKE_C_STANDARD 17)\n";
        assert_eq!(parse_cmake_var(content, "CMAKE_CXX_STANDARD"), Some("20".to_string()));
        assert_eq!(parse_cmake_var(content, "CMAKE_C_STANDARD"), Some("17".to_string()));
    }

    #[test]
    fn missing_var_is_none() {
        let content = "set(CMAKE_CXX_STANDARD 20)\n";
        assert_eq!(parse_cmake_var(content, "CMAKE_OBJC_STANDARD"), None);
    }

    #[test]
    fn replacing_an_existing_var_edits_it_in_place() {
        let content = "set(CMAKE_CXX_STANDARD 20)\nset(CMAKE_C_STANDARD 17)\n";
        let out = replace_cmake_var(content, "CMAKE_C_STANDARD", "23");
        assert_eq!(parse_cmake_var(&out, "CMAKE_C_STANDARD"), Some("23".to_string()));
        // untouched, and not duplicated
        assert_eq!(parse_cmake_var(&out, "CMAKE_CXX_STANDARD"), Some("20".to_string()));
        assert_eq!(out.matches("set(CMAKE_C_STANDARD").count(), 1);
    }

    /// A CMakeLists.txt scaffolded on Windows has no OBJC block at all;
    /// setting the standard there used to report success and write nothing.
    #[test]
    fn setting_an_absent_var_appends_it() {
        let content = "set(CMAKE_CXX_STANDARD 20)\n";
        let out = replace_cmake_var(content, "CMAKE_OBJC_STANDARD", "17");
        assert_eq!(parse_cmake_var(&out, "CMAKE_OBJC_STANDARD"), Some("17".to_string()));
        assert_eq!(parse_cmake_var(&out, "CMAKE_CXX_STANDARD"), Some("20".to_string()));
    }

    /// Locks in the CMake-generation support the `--template lib`/`test`/
    /// `server` templates depend on: a `.cforge_lib` marker builds a
    /// library instead of an executable, a `*_test` app dir is registered
    /// with `add_test`, and ws2_32 is linked on Windows for the socket
    /// code in `--template server`.
    #[test]
    fn generated_cmakelists_supports_lib_test_and_server_templates() {
        let out = generate_cmakelists("demo");
        assert!(out.contains(".cforge_lib"));
        assert!(out.contains("add_library(${app_name} ${lib_kind}"));
        assert!(out.contains("add_test(NAME ${app_name} COMMAND ${app_name})"));
        assert!(out.contains("ws2_32"));
        assert!(out.contains("ALL_LIB_TARGETS"));
    }
}
