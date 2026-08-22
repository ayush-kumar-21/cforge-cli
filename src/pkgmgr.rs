//! Cross-platform library installation and pkg-config resolution.
//! Handles package name variations across Linux, macOS, Windows.
use crate::platform::{command_exists, install_pkg_ok, os, status, Os};
use std::fs;
use std::path::Path;

/// Maps a library name to the package names that provide it on each
/// platform, in preference order. Example: "sqlite3" -> "libsqlite3-dev" on
/// Debian, "sqlite3" on macOS. Development packages (headers) are what's
/// listed, since the point is compiling against the library.
///
/// A `static` table of borrowed slices rather than a function building
/// `Vec<String>`s: this is looked up once per `cforge add`, and rebuilding
/// ~50 structs with ~180 heap allocations to answer one name lookup was
/// pure waste.
pub struct LibraryMapping {
    pub name: &'static str,
    pub linux: &'static [&'static str],
    pub macos: &'static [&'static str],
    pub windows: &'static [&'static str],
}

// One line per library is the point of this table — it is read as rows,
// scanning down the `name` column. rustfmt would explode each of the ~50
// entries into a six-line block and turn a scannable table into 300 lines.
#[rustfmt::skip]
static KNOWN_MAPPINGS: &[LibraryMapping] = &[
    // Core / data formats
    LibraryMapping { name: "sqlite3", linux: &["libsqlite3-dev", "sqlite3-dev", "sqlite3"], macos: &["sqlite3"], windows: &["sqlite3"] },
    LibraryMapping { name: "curl", linux: &["libcurl4-openssl-dev", "libcurl-dev", "curl-dev"], macos: &["curl"], windows: &["curl"] },
    LibraryMapping { name: "openssl", linux: &["libssl-dev", "openssl-dev"], macos: &["openssl"], windows: &["openssl"] },
    LibraryMapping { name: "zlib", linux: &["zlib1g-dev", "zlib-dev"], macos: &["zlib"], windows: &["zlib"] },
    LibraryMapping { name: "json", linux: &["nlohmann-json3-dev", "json-c-dev"], macos: &["nlohmann-json"], windows: &["nlohmann-json"] },
    LibraryMapping { name: "libuuid", linux: &["uuid-dev"], macos: &["ossp-uuid"], windows: &[] },
    LibraryMapping { name: "libxml2", linux: &["libxml2-dev"], macos: &["libxml2"], windows: &["libxml2"] },
    LibraryMapping { name: "yaml-cpp", linux: &["libyaml-cpp-dev"], macos: &["yaml-cpp"], windows: &["yaml-cpp"] },
    LibraryMapping { name: "protobuf", linux: &["libprotobuf-dev", "protobuf-compiler"], macos: &["protobuf"], windows: &["protobuf"] },
    LibraryMapping { name: "msgpack", linux: &["libmsgpack-dev"], macos: &["msgpack-cxx"], windows: &["msgpack"] },
    // Image / graphics
    LibraryMapping { name: "libpng", linux: &["libpng-dev"], macos: &["libpng"], windows: &["libpng"] },
    LibraryMapping { name: "libjpeg", linux: &["libjpeg-dev", "libjpeg-turbo8-dev"], macos: &["jpeg-turbo"], windows: &["libjpeg-turbo"] },
    LibraryMapping { name: "freetype", linux: &["libfreetype-dev", "libfreetype6-dev"], macos: &["freetype"], windows: &["freetype"] },
    LibraryMapping { name: "sdl2", linux: &["libsdl2-dev"], macos: &["sdl2"], windows: &["sdl2"] },
    LibraryMapping { name: "glfw3", linux: &["libglfw3-dev"], macos: &["glfw"], windows: &["glfw3"] },
    LibraryMapping { name: "opengl", linux: &["libgl1-mesa-dev", "mesa-common-dev"], macos: &[], windows: &["opengl"] },
    LibraryMapping { name: "cairo", linux: &["libcairo2-dev"], macos: &["cairo"], windows: &["cairo"] },
    LibraryMapping { name: "vulkan", linux: &["libvulkan-dev", "vulkan-sdk"], macos: &["vulkan-headers", "molten-vk"], windows: &["vulkan"] },
    // Networking / async
    LibraryMapping { name: "libuv", linux: &["libuv1-dev"], macos: &["libuv"], windows: &["libuv"] },
    LibraryMapping { name: "zeromq", linux: &["libzmq3-dev"], macos: &["zeromq"], windows: &["zeromq"] },
    LibraryMapping { name: "grpc", linux: &["libgrpc++-dev"], macos: &["grpc"], windows: &["grpc"] },
    LibraryMapping { name: "asio", linux: &["libasio-dev"], macos: &["asio"], windows: &["asio"] },
    LibraryMapping { name: "libssh2", linux: &["libssh2-1-dev"], macos: &["libssh2"], windows: &["libssh2"] },
    LibraryMapping { name: "libwebsockets", linux: &["libwebsockets-dev"], macos: &["libwebsockets"], windows: &["libwebsockets"] },
    // Compression
    LibraryMapping { name: "bzip2", linux: &["libbz2-dev"], macos: &["bzip2"], windows: &["bzip2"] },
    LibraryMapping { name: "lz4", linux: &["liblz4-dev"], macos: &["lz4"], windows: &["lz4"] },
    LibraryMapping { name: "zstd", linux: &["libzstd-dev"], macos: &["zstd"], windows: &["zstd"] },
    LibraryMapping { name: "libarchive", linux: &["libarchive-dev"], macos: &["libarchive"], windows: &["libarchive"] },
    // Testing / utility
    LibraryMapping { name: "gtest", linux: &["libgtest-dev"], macos: &["googletest"], windows: &["gtest"] },
    LibraryMapping { name: "catch2", linux: &["catch2"], macos: &["catch2"], windows: &["catch2"] },
    LibraryMapping { name: "fmt", linux: &["libfmt-dev"], macos: &["fmt"], windows: &["fmt"] },
    LibraryMapping { name: "spdlog", linux: &["libspdlog-dev"], macos: &["spdlog"], windows: &["spdlog"] },
    LibraryMapping { name: "boost", linux: &["libboost-all-dev"], macos: &["boost"], windows: &["boost"] },
    LibraryMapping { name: "abseil", linux: &["libabsl-dev"], macos: &["abseil"], windows: &["abseil"] },
    LibraryMapping { name: "tbb", linux: &["libtbb-dev"], macos: &["tbb"], windows: &["tbb"] },
    // Database clients
    LibraryMapping { name: "postgresql", linux: &["libpq-dev"], macos: &["libpq"], windows: &["libpq"] },
    LibraryMapping { name: "mysql", linux: &["libmysqlclient-dev", "default-libmysqlclient-dev"], macos: &["mysql-client"], windows: &["libmysql"] },
    LibraryMapping { name: "hiredis", linux: &["libhiredis-dev"], macos: &["hiredis"], windows: &["hiredis"] },
    LibraryMapping { name: "mongo-c-driver", linux: &["libmongoc-dev"], macos: &["mongo-c-driver"], windows: &["mongo-c-driver"] },
    // Math / scientific
    LibraryMapping { name: "eigen3", linux: &["libeigen3-dev"], macos: &["eigen"], windows: &["eigen3"] },
    LibraryMapping { name: "gmp", linux: &["libgmp-dev"], macos: &["gmp"], windows: &["gmp"] },
    LibraryMapping { name: "fftw3", linux: &["libfftw3-dev"], macos: &["fftw"], windows: &["fftw3"] },
    LibraryMapping { name: "openblas", linux: &["libopenblas-dev"], macos: &["openblas"], windows: &["openblas"] },
    LibraryMapping { name: "lapack", linux: &["liblapack-dev"], macos: &["lapack"], windows: &["lapack"] },
    // Misc system
    LibraryMapping { name: "readline", linux: &["libreadline-dev"], macos: &["readline"], windows: &[] },
    LibraryMapping { name: "ncurses", linux: &["libncurses-dev", "libncurses5-dev"], macos: &["ncurses"], windows: &["pdcurses"] },
    LibraryMapping { name: "pcre2", linux: &["libpcre2-dev"], macos: &["pcre2"], windows: &["pcre2"] },
    LibraryMapping { name: "icu", linux: &["libicu-dev"], macos: &["icu4c"], windows: &["icu"] },
    LibraryMapping { name: "double-conversion", linux: &["libdouble-conversion-dev"], macos: &["double-conversion"], windows: &["double-conversion"] },
    LibraryMapping { name: "pthread", linux: &[], macos: &[], windows: &[] },
];

/// Get candidate package names for a library on this platform.
fn get_package_candidates(libname: &str) -> &'static [&'static str] {
    match KNOWN_MAPPINGS.iter().find(|m| m.name == libname) {
        Some(m) => match os() {
            Os::Linux => m.linux,
            Os::Macos => m.macos,
            Os::Windows => m.windows,
            Os::Unknown => &[],
        },
        // Unmapped library: the name itself is the best guess. Returned as a
        // borrowed slice via the caller, which falls back to `libname`.
        None => &[],
    }
}

/// Installs a library, trying each known package name for this platform in
/// order and returning the one that worked (recorded to deps.lock so a
/// teammate/CI doesn't repeat the guessing).
///
/// Uses `install_pkg_ok`, not `install_pkg`: the fatal variant exited the
/// process the moment a candidate failed, so every name after the first was
/// unreachable — on a distro where the first guess is wrong (`dnf install
/// libcurl4-openssl-dev`), `cforge add curl` died instead of trying
/// `curl-dev`.
pub fn install_library(libname: &str) -> Option<String> {
    let mapped = get_package_candidates(libname);
    let candidates: Vec<&str> = if mapped.is_empty() { vec![libname] } else { mapped.to_vec() };

    // Without pkg-config there is nothing to verify against, so a successful
    // install is the only signal available. Treating it as failure (the old
    // behaviour) made `cforge add` report an error on every Windows box and
    // on any Unix without pkg-config, after having installed the package
    // correctly.
    let can_verify = command_exists("pkg-config");

    for pkg_name in candidates {
        status(&format!("Trying to install {pkg_name}..."));
        if !install_pkg_ok(pkg_name) {
            continue;
        }
        if !can_verify || verify_pkg_config(libname) {
            return Some(pkg_name.to_string());
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
    let output = std::process::Command::new("pkg-config").args(["--exists", libname]).status();
    output.map(|s| s.success()).unwrap_or(false)
}

/// Add a library to libs.txt.
pub fn add_to_libs(libname: &str) {
    let libs_file = Path::new("libs.txt");
    let mut libs: Vec<String> =
        fs::read_to_string(libs_file).map(|s| s.lines().map(|l| l.to_string()).collect()).unwrap_or_default();

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
    let mut libs: Vec<String> =
        fs::read_to_string(libs_file).map(|s| s.lines().map(|l| l.to_string()).collect()).unwrap_or_default();

    libs.retain(|l| l != libname);
    if libs.is_empty() {
        fs::remove_file(libs_file).ok();
    } else {
        let content = libs.iter().map(|l| format!("{l}\n")).collect::<String>();
        crate::platform::write_file(libs_file, &content);
    }
}
