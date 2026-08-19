use crate::config::Config;
use crate::platform::{c_compiler, cxx_compiler, probe_std};
use std::fs;
use std::path::Path;

/// In-place replacement of a `set(VAR <value>)` line in CMakeLists.txt.
/// Rust does this with plain string ops instead of shelling out to `sed`,
/// which sidesteps the BSD-vs-GNU `sed -i` incompatibility entirely.
pub fn set_cmake_var(path: &Path, var: &str, val: &str) {
    let content = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("Error: could not read {}: {e}", path.display());
        std::process::exit(1);
    });
    let prefix = format!("set({var} ");
    let new_content: String = content
        .lines()
        .map(|line| {
            if line.trim_start().starts_with(&prefix) {
                format!("set({var} {val})")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    crate::platform::write_file(path, &(new_content + "\n"));
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
    let resolved = resolve_std(val, latest_cxx_std, "C++", |v| probe_std(cxx_compiler(), &format!("-std=c++{v}"), "c++"));
    set_cmake_var(cmake_file, "CMAKE_CXX_STANDARD", &resolved);
    crate::platform::status(&format!("Pinned CMAKE_CXX_STANDARD={resolved} in CMakeLists.txt"));
}

pub fn set_objc_std(cmake_file: &Path, val: &str) {
    if !crate::project::objc_capable_platform() {
        eprintln!("Error: Objective-C is not supported on this platform (no viable toolchain).");
        std::process::exit(3);
    }
    let resolved = resolve_std(val, latest_objc_std, "Objective-C", |v| probe_std(c_compiler(), &format!("-std=c{v}"), "objective-c"));
    set_cmake_var(cmake_file, "CMAKE_OBJC_STANDARD", &resolved);
    crate::platform::status(&format!("Pinned CMAKE_OBJC_STANDARD={resolved} in CMakeLists.txt"));
}

pub fn set_objcxx_std(cmake_file: &Path, val: &str) {
    if !crate::project::objc_capable_platform() {
        eprintln!("Error: Objective-C++ is not supported on this platform (no viable toolchain).");
        std::process::exit(3);
    }
    let resolved = resolve_std(val, latest_objcxx_std, "Objective-C++", |v| probe_std(cxx_compiler(), &format!("-std=c++{v}"), "objective-c++"));
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
    set(CFORGE_SWIFT_SRC "Swift")
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
# Swift is enabled here (rather than in the initial project() LANGUAGES,
# like C/CXX/OBJC/OBJCXX above) precisely so a project that never opts in
# via `cforge lang add swift` never needs a Swift compiler to configure at
# all. CMake's Swift support also only works with the Ninja or Xcode
# generators, which `cforge build` only forces once Swift is enabled.
if("swift" IN_LIST ENABLED_LANGS)
    enable_language(Swift)
    file(GLOB SWIFT_FILES "${CFORGE_SWIFT_SRC}/*.swift")
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

function(add_lang_executables files suffix needs_foundation)
    foreach(source_file ${files})
        get_filename_component(base_name ${source_file} NAME_WE)
        set(exec_name "${base_name}_${suffix}")
        add_executable(${exec_name} ${source_file})
        if(needs_foundation)
            if(APPLE)
                target_link_libraries(${exec_name} "-framework Foundation")
            else()
                # Linux Obj-C/Obj-C++ has no Foundation; GNUstep's base
                # library is the closest equivalent. `cforge toolchain
                # install obj_c` (Linux) installs gnustep-base and
                # gnustep-config alongside it.
                find_program(GNUSTEP_CONFIG gnustep-config)
                if(GNUSTEP_CONFIG)
                    execute_process(COMMAND ${GNUSTEP_CONFIG} --objc-flags
                        OUTPUT_VARIABLE GNUSTEP_OBJC_FLAGS OUTPUT_STRIP_TRAILING_WHITESPACE)
                    execute_process(COMMAND ${GNUSTEP_CONFIG} --base-libs
                        OUTPUT_VARIABLE GNUSTEP_BASE_LIBS OUTPUT_STRIP_TRAILING_WHITESPACE)
                    separate_arguments(GNUSTEP_OBJC_FLAGS_LIST UNIX_COMMAND "${GNUSTEP_OBJC_FLAGS}")
                    separate_arguments(GNUSTEP_BASE_LIBS_LIST UNIX_COMMAND "${GNUSTEP_BASE_LIBS}")
                    target_compile_options(${exec_name} PRIVATE ${GNUSTEP_OBJC_FLAGS_LIST})
                    target_link_libraries(${exec_name} ${GNUSTEP_BASE_LIBS_LIST})
                else()
                    message(WARNING "gnustep-config not found; ${exec_name} will not link GNUstep base. Run 'cforge toolchain install obj_c' first.")
                endif()
            endif()
        endif()
        if(EXTRA_LIB_TARGETS)
            target_link_libraries(${exec_name} ${EXTRA_LIB_TARGETS})
        endif()
        list(APPEND ALL_EXEC_TARGETS ${exec_name})
    endforeach()
    set(ALL_EXEC_TARGETS ${ALL_EXEC_TARGETS} PARENT_SCOPE)
endfunction()

add_lang_executables("${C_FILES}" c FALSE)
add_lang_executables("${CPP_FILES}" cpp FALSE)
add_lang_executables("${OBJC_FILES}" objc TRUE)
add_lang_executables("${OBJCPP_FILES}" objcpp TRUE)
add_lang_executables("${SWIFT_FILES}" swift FALSE)

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
         set(CFORGE_SWIFT_SRC \"{}\")\n\
         set(CFORGE_HEADERS \"{}\")\n",
        cfg.src_dir("c"),
        cfg.src_dir("cpp"),
        cfg.src_dir("obj_c"),
        cfg.src_dir("obj_cpp"),
        cfg.src_dir("swift"),
        cfg.paths.headers,
    );
    crate::platform::write_file(Path::new(".cforge_config.cmake"), &content);
}

/// Builds the CMakeLists.txt content for a fresh project. OBJC/OBJCXX are
/// only requested as project() languages on macOS and Linux (GNUstep): on
/// Windows, CMake would hard-fail configuring even a pure-C project while
/// hunting for an Objective-C compiler that doesn't exist there.
pub fn generate_cmakelists(project_name: &str) -> String {
    let objc_ok = crate::project::objc_capable_platform();
    let proj_langs = if objc_ok { "C CXX OBJC OBJCXX" } else { "C CXX" };

    let mut out = format!(
        "cmake_minimum_required(VERSION 3.16)\nproject({project_name} VERSION 0.1.0 LANGUAGES {proj_langs})\n\n"
    );
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
}
