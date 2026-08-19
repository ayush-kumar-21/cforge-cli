//! Cross-platform library installation and pkg-config resolution.
//! Handles package name variations across Linux, macOS, Windows.
use crate::platform::{command_exists, install_pkg, os, status, Os};
use std::fs;
use std::path::Path;

/// Maps a library name to potential package names for each platform.
/// Example: "sqlite3" -> ["sqlite3", "libsqlite3-dev"] on Linux, ["sqlite3"] on macOS.
pub struct LibraryMapping {
    pub name: String,
    pub linux_names: Vec<&'static str>,
    pub macos_names: Vec<&'static str>,
    pub windows_names: Vec<&'static str>,
}

/// Common library mappings. This is a starting point; real projects would
/// need a much larger database or a network lookup service. Mappings include
/// the development package name needed to compile against the library.
fn known_mappings() -> Vec<LibraryMapping> {
    vec![
        // Core / data formats
        LibraryMapping { name: "sqlite3".to_string(), linux_names: vec!["libsqlite3-dev", "sqlite3-dev", "sqlite3"], macos_names: vec!["sqlite3"], windows_names: vec!["sqlite3"] },
        LibraryMapping { name: "curl".to_string(), linux_names: vec!["libcurl4-openssl-dev", "libcurl-dev", "curl-dev"], macos_names: vec!["curl"], windows_names: vec!["curl"] },
        LibraryMapping { name: "openssl".to_string(), linux_names: vec!["libssl-dev", "openssl-dev"], macos_names: vec!["openssl"], windows_names: vec!["openssl"] },
        LibraryMapping { name: "zlib".to_string(), linux_names: vec!["zlib1g-dev", "zlib-dev"], macos_names: vec!["zlib"], windows_names: vec!["zlib"] },
        LibraryMapping { name: "json".to_string(), linux_names: vec!["nlohmann-json3-dev", "json-c-dev"], macos_names: vec!["nlohmann-json"], windows_names: vec!["nlohmann-json"] },
        LibraryMapping { name: "libuuid".to_string(), linux_names: vec!["uuid-dev"], macos_names: vec!["ossp-uuid"], windows_names: vec![] },
        LibraryMapping { name: "libxml2".to_string(), linux_names: vec!["libxml2-dev"], macos_names: vec!["libxml2"], windows_names: vec!["libxml2"] },
        LibraryMapping { name: "yaml-cpp".to_string(), linux_names: vec!["libyaml-cpp-dev"], macos_names: vec!["yaml-cpp"], windows_names: vec!["yaml-cpp"] },
        LibraryMapping { name: "protobuf".to_string(), linux_names: vec!["libprotobuf-dev", "protobuf-compiler"], macos_names: vec!["protobuf"], windows_names: vec!["protobuf"] },
        LibraryMapping { name: "msgpack".to_string(), linux_names: vec!["libmsgpack-dev"], macos_names: vec!["msgpack-cxx"], windows_names: vec!["msgpack"] },

        // Image / graphics
        LibraryMapping { name: "libpng".to_string(), linux_names: vec!["libpng-dev"], macos_names: vec!["libpng"], windows_names: vec!["libpng"] },
        LibraryMapping { name: "libjpeg".to_string(), linux_names: vec!["libjpeg-dev", "libjpeg-turbo8-dev"], macos_names: vec!["jpeg-turbo"], windows_names: vec!["libjpeg-turbo"] },
        LibraryMapping { name: "freetype".to_string(), linux_names: vec!["libfreetype-dev", "libfreetype6-dev"], macos_names: vec!["freetype"], windows_names: vec!["freetype"] },
        LibraryMapping { name: "sdl2".to_string(), linux_names: vec!["libsdl2-dev"], macos_names: vec!["sdl2"], windows_names: vec!["sdl2"] },
        LibraryMapping { name: "glfw3".to_string(), linux_names: vec!["libglfw3-dev"], macos_names: vec!["glfw"], windows_names: vec!["glfw3"] },
        LibraryMapping { name: "opengl".to_string(), linux_names: vec!["libgl1-mesa-dev", "mesa-common-dev"], macos_names: vec![], windows_names: vec!["opengl"] },
        LibraryMapping { name: "cairo".to_string(), linux_names: vec!["libcairo2-dev"], macos_names: vec!["cairo"], windows_names: vec!["cairo"] },
        LibraryMapping { name: "vulkan".to_string(), linux_names: vec!["libvulkan-dev", "vulkan-sdk"], macos_names: vec!["vulkan-headers", "molten-vk"], windows_names: vec!["vulkan"] },

        // Networking / async
        LibraryMapping { name: "libuv".to_string(), linux_names: vec!["libuv1-dev"], macos_names: vec!["libuv"], windows_names: vec!["libuv"] },
        LibraryMapping { name: "zeromq".to_string(), linux_names: vec!["libzmq3-dev"], macos_names: vec!["zeromq"], windows_names: vec!["zeromq"] },
        LibraryMapping { name: "grpc".to_string(), linux_names: vec!["libgrpc++-dev"], macos_names: vec!["grpc"], windows_names: vec!["grpc"] },
        LibraryMapping { name: "asio".to_string(), linux_names: vec!["libasio-dev"], macos_names: vec!["asio"], windows_names: vec!["asio"] },
        LibraryMapping { name: "libssh2".to_string(), linux_names: vec!["libssh2-1-dev"], macos_names: vec!["libssh2"], windows_names: vec!["libssh2"] },
        LibraryMapping { name: "libwebsockets".to_string(), linux_names: vec!["libwebsockets-dev"], macos_names: vec!["libwebsockets"], windows_names: vec!["libwebsockets"] },

        // Compression
        LibraryMapping { name: "bzip2".to_string(), linux_names: vec!["libbz2-dev"], macos_names: vec!["bzip2"], windows_names: vec!["bzip2"] },
        LibraryMapping { name: "lz4".to_string(), linux_names: vec!["liblz4-dev"], macos_names: vec!["lz4"], windows_names: vec!["lz4"] },
        LibraryMapping { name: "zstd".to_string(), linux_names: vec!["libzstd-dev"], macos_names: vec!["zstd"], windows_names: vec!["zstd"] },
        LibraryMapping { name: "libarchive".to_string(), linux_names: vec!["libarchive-dev"], macos_names: vec!["libarchive"], windows_names: vec!["libarchive"] },

        // Testing / utility
        LibraryMapping { name: "gtest".to_string(), linux_names: vec!["libgtest-dev"], macos_names: vec!["googletest"], windows_names: vec!["gtest"] },
        LibraryMapping { name: "catch2".to_string(), linux_names: vec!["catch2"], macos_names: vec!["catch2"], windows_names: vec!["catch2"] },
        LibraryMapping { name: "fmt".to_string(), linux_names: vec!["libfmt-dev"], macos_names: vec!["fmt"], windows_names: vec!["fmt"] },
        LibraryMapping { name: "spdlog".to_string(), linux_names: vec!["libspdlog-dev"], macos_names: vec!["spdlog"], windows_names: vec!["spdlog"] },
        LibraryMapping { name: "boost".to_string(), linux_names: vec!["libboost-all-dev"], macos_names: vec!["boost"], windows_names: vec!["boost"] },
        LibraryMapping { name: "abseil".to_string(), linux_names: vec!["libabsl-dev"], macos_names: vec!["abseil"], windows_names: vec!["abseil"] },
        LibraryMapping { name: "tbb".to_string(), linux_names: vec!["libtbb-dev"], macos_names: vec!["tbb"], windows_names: vec!["tbb"] },

        // Database clients
        LibraryMapping { name: "postgresql".to_string(), linux_names: vec!["libpq-dev"], macos_names: vec!["libpq"], windows_names: vec!["libpq"] },
        LibraryMapping { name: "mysql".to_string(), linux_names: vec!["libmysqlclient-dev", "default-libmysqlclient-dev"], macos_names: vec!["mysql-client"], windows_names: vec!["libmysql"] },
        LibraryMapping { name: "hiredis".to_string(), linux_names: vec!["libhiredis-dev"], macos_names: vec!["hiredis"], windows_names: vec!["hiredis"] },
        LibraryMapping { name: "mongo-c-driver".to_string(), linux_names: vec!["libmongoc-dev"], macos_names: vec!["mongo-c-driver"], windows_names: vec!["mongo-c-driver"] },

        // Math / scientific
        LibraryMapping { name: "eigen3".to_string(), linux_names: vec!["libeigen3-dev"], macos_names: vec!["eigen"], windows_names: vec!["eigen3"] },
        LibraryMapping { name: "gmp".to_string(), linux_names: vec!["libgmp-dev"], macos_names: vec!["gmp"], windows_names: vec!["gmp"] },
        LibraryMapping { name: "fftw3".to_string(), linux_names: vec!["libfftw3-dev"], macos_names: vec!["fftw"], windows_names: vec!["fftw3"] },
        LibraryMapping { name: "openblas".to_string(), linux_names: vec!["libopenblas-dev"], macos_names: vec!["openblas"], windows_names: vec!["openblas"] },
        LibraryMapping { name: "lapack".to_string(), linux_names: vec!["liblapack-dev"], macos_names: vec!["lapack"], windows_names: vec!["lapack"] },

        // Misc system
        LibraryMapping { name: "readline".to_string(), linux_names: vec!["libreadline-dev"], macos_names: vec!["readline"], windows_names: vec![] },
        LibraryMapping { name: "ncurses".to_string(), linux_names: vec!["libncurses-dev", "libncurses5-dev"], macos_names: vec!["ncurses"], windows_names: vec!["pdcurses"] },
        LibraryMapping { name: "pcre2".to_string(), linux_names: vec!["libpcre2-dev"], macos_names: vec!["pcre2"], windows_names: vec!["pcre2"] },
        LibraryMapping { name: "icu".to_string(), linux_names: vec!["libicu-dev"], macos_names: vec!["icu4c"], windows_names: vec!["icu"] },
        LibraryMapping { name: "double-conversion".to_string(), linux_names: vec!["libdouble-conversion-dev"], macos_names: vec!["double-conversion"], windows_names: vec!["double-conversion"] },
        LibraryMapping { name: "pthread".to_string(), linux_names: vec![], macos_names: vec![], windows_names: vec![] },
    ]
}

/// Get candidate package names for a library on this platform.
fn get_package_candidates(libname: &str) -> Vec<String> {
    for mapping in known_mappings() {
        if mapping.name == libname {
            let names = match os() {
                Os::Linux => mapping.linux_names,
                Os::Macos => mapping.macos_names,
                Os::Windows => mapping.windows_names,
                Os::Unknown => vec![],
            };
            return names.into_iter().map(|s| s.to_string()).collect();
        }
    }
    // Fallback: try the name itself
    vec![libname.to_string()]
}

/// Try to install a library, attempting known package names in order.
/// On success, returns the package name that actually worked (recorded to
/// deps.lock so a teammate/CI doesn't have to repeat the name-guessing).
pub fn install_library(libname: &str) -> Option<String> {
    let candidates = get_package_candidates(libname);

    for pkg_name in candidates {
        status(&format!("Trying to install {pkg_name}..."));
        // Try to install — if it fails, continue to next candidate
        install_pkg(&pkg_name);

        // Verify pkg-config can find the library
        if verify_pkg_config(libname) {
            return Some(pkg_name);
        }
    }

    None
}

/// The platform key used in deps.lock — a plain lowercase string is easier
/// to hand-read/hand-edit in a lockfile than the internal `Os` enum's Debug
/// form, and stays stable if `Os`'s variant names ever change.
pub fn platform_key() -> &'static str {
    match os() {
        Os::Linux => "linux",
        Os::Macos => "macos",
        Os::Windows => "windows",
        Os::Unknown => "unknown",
    }
}

/// Check if pkg-config can find a library.
pub fn verify_pkg_config(libname: &str) -> bool {
    if !command_exists("pkg-config") {
        return false;
    }
    let output = std::process::Command::new("pkg-config")
        .args(["--exists", libname])
        .status();
    output.map(|s| s.success()).unwrap_or(false)
}

/// Add a library to libs.txt.
pub fn add_to_libs(libname: &str) {
    let libs_file = Path::new("libs.txt");
    let mut libs: Vec<String> = fs::read_to_string(libs_file)
        .map(|s| s.lines().map(|l| l.to_string()).collect())
        .unwrap_or_default();

    if !libs.iter().any(|l| l == libname) {
        libs.push(libname.to_string());
        libs.sort();
        let content = libs.iter().map(|l| format!("{l}\n")).collect::<String>();
        crate::platform::write_file(libs_file, &content);
    }
}

/// Remove a library from libs.txt.
pub fn remove_from_libs(libname: &str) {
    let libs_file = Path::new("libs.txt");
    let mut libs: Vec<String> = fs::read_to_string(libs_file)
        .map(|s| s.lines().map(|l| l.to_string()).collect())
        .unwrap_or_default();

    libs.retain(|l| l != libname);
    if libs.is_empty() {
        fs::remove_file(libs_file).ok();
    } else {
        let content = libs.iter().map(|l| format!("{l}\n")).collect::<String>();
        crate::platform::write_file(libs_file, &content);
    }
}
