use crate::platform::{command_exists, home_dir, os, Os};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn install_dir() -> PathBuf {
    home_dir().join(".local").join("bin")
}

fn installed_binary_name() -> &'static str {
    if os() == Os::Windows { "cforge.exe" } else { "cforge" }
}

fn installed_path() -> PathBuf {
    // Prefer wherever the running binary actually is; fall back to the
    // conventional install dir if that can't be resolved.
    std::env::current_exe().unwrap_or_else(|_| install_dir().join(installed_binary_name()))
}

fn source_dir() -> PathBuf {
    std::env::var("CFORGE_SOURCE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home_dir().join("Documents").join("CodingProjects").join("cforge"))
}

const REPO: &str = "ayush-kumar-21/cforge-cli";

/// Matches the asset names produced by .github/workflows/release.yml.
fn release_asset_name() -> Option<&'static str> {
    match (os(), std::env::consts::ARCH) {
        (Os::Macos, "aarch64") => Some("cforge-macos-arm64"),
        (Os::Macos, "x86_64") => Some("cforge-macos-x86_64"),
        (Os::Linux, "x86_64") => Some("cforge-linux-x86_64"),
        (Os::Windows, "x86_64") => Some("cforge-windows-x86_64.exe"),
        _ => None,
    }
}

fn download_asset(asset: &str, dest: &std::path::Path) -> bool {
    let url = format!("https://github.com/{REPO}/releases/latest/download/{asset}");
    match os() {
        Os::Windows => {
            let script = format!("Invoke-WebRequest -Uri '{url}' -OutFile '{}'", dest.display());
            Command::new("powershell")
                .args(["-NoProfile", "-Command", &script])
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        }
        _ => {
            if !command_exists("curl") {
                return false;
            }
            Command::new("curl")
                .args(["-fsSL", &url, "-o"])
                .arg(dest)
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        }
    }
}

/// Primary update path: download the matching binary from the latest
/// GitHub release, same idea as `uv self update`. Returns false (rather
/// than exiting) so the caller can fall back to a local source rebuild.
fn try_update_from_release() -> bool {
    let Some(asset) = release_asset_name() else {
        return false;
    };
    let dest = installed_path();
    let tmp = dest.with_extension("new");

    println!("Downloading latest release ({asset})...");
    if !download_asset(asset, &tmp) {
        eprintln!("Warning: could not download the latest release; trying a local source rebuild instead.");
        fs::remove_file(&tmp).ok();
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(mut perm) = fs::metadata(&tmp).map(|m| m.permissions()) {
            perm.set_mode(0o755);
            fs::set_permissions(&tmp, perm).ok();
        }
        fs::rename(&tmp, &dest).unwrap_or_else(|e| {
            eprintln!("Error: could not install downloaded binary: {e}");
            std::process::exit(1);
        });
        println!("Updated {} to the latest release.", dest.display());
    }
    #[cfg(windows)]
    {
        // A running .exe can't be overwritten directly on Windows. Hand the
        // swap to a detached helper that runs after this process exits.
        let dest_str = dest.to_string_lossy().to_string();
        let tmp_str = tmp.to_string_lossy().to_string();
        let script = format!("Start-Sleep -Milliseconds 500; Move-Item -Force '{tmp_str}' '{dest_str}'");
        Command::new("powershell")
            .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script])
            .spawn()
            .ok();
        println!("Downloaded the latest release. Finishing install in the background (re-run 'cforge' in a moment).");
    }
    true
}

fn update_from_source() {
    let src_dir = source_dir();
    if !src_dir.join("Cargo.toml").is_file() {
        eprintln!(
            "Error: no local source checkout at {} either (set CFORGE_SOURCE to override).",
            src_dir.display()
        );
        std::process::exit(1);
    }
    if !command_exists("cargo") {
        eprintln!("Error: cargo not found. Install Rust (https://rustup.rs) and re-run 'cforge self-update'.");
        std::process::exit(1);
    }

    println!("Building from local source at {}...", src_dir.display());
    let status = Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(&src_dir)
        .status();
    if !status.map(|s| s.success()).unwrap_or(false) {
        eprintln!("Error: build failed.");
        std::process::exit(1);
    }

    let built = src_dir.join("target").join("release").join(installed_binary_name());
    let dest = installed_path();
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::copy(&built, &dest).unwrap_or_else(|e| {
        eprintln!("Error: could not copy {} to {}: {e}", built.display(), dest.display());
        std::process::exit(1);
    });
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(mut perm) = fs::metadata(&dest).map(|m| m.permissions()) {
            perm.set_mode(0o755);
            fs::set_permissions(&dest, perm).ok();
        }
    }
    println!("Updated {} from {}", dest.display(), built.display());
}

pub fn update_self() {
    if try_update_from_release() {
        return;
    }
    update_from_source();
}

#[cfg(not(target_os = "windows"))]
fn remove_path_entries() {
    let marker = "# cforge PATH";
    let export_line = "export PATH=\"$HOME/.local/bin:$PATH\"";
    for rc in ["/.zshrc", "/.bashrc", "/.bash_profile", "/.profile"] {
        let path = home_dir().join(rc.trim_start_matches('/'));
        let Ok(content) = fs::read_to_string(&path) else { continue };
        if !content.contains(marker) {
            continue;
        }
        let cleaned: String = content
            .lines()
            .filter(|l| *l != marker && *l != export_line)
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(&path, cleaned + "\n").ok();
        println!("Removed cforge PATH entry from {}", path.display());
    }
}

#[cfg(target_os = "windows")]
fn remove_path_entries() {
    let dir = install_dir();
    let dir_str = dir.to_string_lossy().to_string();
    let script = format!(
        "$dir = '{}'; \
         $cur = [Environment]::GetEnvironmentVariable('Path','User'); \
         if ($cur -like \"*${{dir}}*\") {{ \
           $new = ($cur -split ';' | Where-Object {{ $_ -ne $dir }}) -join ';'; \
           [Environment]::SetEnvironmentVariable('Path', $new, 'User'); \
           Write-Host \"Removed $dir from User PATH\" \
         }}",
        dir_str
    );
    Command::new("powershell").args(["-NoProfile", "-Command", &script]).status().ok();
}

pub fn uninstall_self() {
    let dest = installed_path();
    if dest.is_file() {
        fs::remove_file(&dest).unwrap_or_else(|e| {
            eprintln!("Error: could not remove {}: {e}", dest.display());
            std::process::exit(1);
        });
        println!("Removed {}", dest.display());
    } else {
        println!("cforge is not installed at {}; nothing to remove there.", dest.display());
    }

    remove_path_entries();

    println!("cforge uninstalled. Your projects and their CMakeLists.txt files are untouched.");
}
