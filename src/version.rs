use crate::platform::{c_compiler, command_path, os, run_capture, Os};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn first_line(s: &str) -> &str {
    s.lines().next().unwrap_or(s)
}

fn print_cmake_line() {
    let version = run_capture("cmake", &["--version"])
        .map(|out| first_line(&out).trim_start_matches("cmake version ").to_string())
        .unwrap_or_else(|| "not found".to_string());
    let path = command_path("cmake").map(|p| p.display().to_string()).unwrap_or_default();
    println!("cmake      {version:<10} {path}");
}

fn print_compiler_line() {
    let compiler = c_compiler();
    let version = run_capture(compiler, &["--version"])
        .map(|out| first_line(&out).to_string())
        .unwrap_or_else(|| "not found".to_string());
    let path = command_path(compiler).map(|p| p.display().to_string()).unwrap_or_default();
    println!("{compiler:<11}{version:<10} {path}");
}

fn print_platform_line() {
    let os_name = match os() {
        Os::Macos => "macOS".to_string(),
        Os::Linux => "Linux".to_string(),
        Os::Windows => "Windows".to_string(),
        Os::Unknown => std::env::consts::OS.to_string(),
    };
    let detail = if os() == Os::Macos {
        run_capture("sw_vers", &["-productVersion"])
    } else {
        None
    };
    match detail {
        Some(v) => println!("platform   {os_name} {v} ({})", std::env::consts::ARCH),
        None => println!("platform   {os_name} ({})", std::env::consts::ARCH),
    }
}

fn print_sdk_line() {
    if os() != Os::Macos {
        return;
    }
    if let Some(v) = run_capture("xcrun", &["--sdk", "macosx", "--show-sdk-version"]) {
        println!("sdk        MacOSX{v}.sdk");
    }
}

pub fn print(verbose: bool) -> ! {
    println!("cforge {VERSION}");
    if verbose {
        print_cmake_line();
        print_compiler_line();
        print_platform_line();
        print_sdk_line();
    }
    std::process::exit(0);
}
