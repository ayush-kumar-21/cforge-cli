use crate::platform::{command_exists, home_dir, os, Os};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn install_dir() -> PathBuf {
    home_dir().join(".local").join("bin")
}

fn installed_binary_name() -> &'static str {
    if os() == Os::Windows {
        "cforge.exe"
    } else {
        "cforge"
    }
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

fn asset_url(asset: &str) -> String {
    format!("https://github.com/{REPO}/releases/latest/download/{asset}")
}

/// Fetches the `<asset>.sha256` published alongside each release binary by
/// release.yml. `None` means it could not be retrieved at all — which is
/// treated as a hard failure by the caller, not as "unverified but fine".
fn download_expected_digest(asset: &str) -> Option<String> {
    let url = asset_url(&format!("{asset}.sha256"));
    let out = match os() {
        Os::Windows => Command::new("powershell")
            .args(["-NoProfile", "-Command", &format!("(Invoke-WebRequest -UseBasicParsing -Uri '{url}').Content")])
            .output()
            .ok()?,
        _ => {
            if !command_exists("curl") {
                return None;
            }
            Command::new("curl").args(["-fsSL", &url]).output().ok()?
        }
    };
    if !out.status.success() {
        return None;
    }
    parse_digest(&String::from_utf8(out.stdout).ok()?)
}

/// Pure parse half of `download_expected_digest`, so the validation can be
/// tested without the network. Tolerates the `<digest>  <filename>` form
/// in case the file is ever regenerated with a plain `sha256sum` redirect,
/// and rejects anything that isn't exactly 64 hex characters — an error
/// page or a truncated download must not read as a valid digest.
fn parse_digest(text: &str) -> Option<String> {
    let digest = text.split_whitespace().next()?.trim().to_ascii_lowercase();
    let valid = digest.len() == 64 && digest.chars().all(|c| c.is_ascii_hexdigit());
    valid.then_some(digest)
}

/// Hashes the downloaded file and compares it to the published digest.
/// Fail-closed in every branch: a missing checksum file, an unreadable
/// download, or a mismatch all return false. Fail-open on a *missing*
/// checksum would be no protection at all — anyone able to substitute the
/// binary over the wire can equally make the checksum fetch 404.
fn verify_download(asset: &str, file: &std::path::Path) -> bool {
    let Some(expected) = download_expected_digest(asset) else {
        eprintln!(
            "Error: could not fetch the published checksum for {asset}.\n\
             Refusing to install an unverified binary. Retry, or download it manually from\n\
             https://github.com/{REPO}/releases/latest and check it yourself."
        );
        return false;
    };
    let Ok(bytes) = fs::read(file) else {
        eprintln!("Error: could not read the downloaded file back for verification.");
        return false;
    };
    let actual = crate::sha256::hex(&bytes);
    if actual != expected {
        eprintln!(
            "Error: checksum mismatch for {asset} — refusing to install.\n\
             \x20 expected {expected}\n\
             \x20 actual   {actual}\n\
             The download was corrupted or tampered with. Nothing has been changed."
        );
        return false;
    }
    true
}

fn download_asset(asset: &str, dest: &std::path::Path) -> bool {
    let url = asset_url(asset);
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
            Command::new("curl").args(["-fsSL", &url, "-o"]).arg(dest).status().map(|s| s.success()).unwrap_or(false)
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

    // Verify before the binary goes anywhere near the install path. A
    // failure here is fatal rather than falling through to the local
    // source rebuild: the download not matching its published checksum
    // means something is wrong with the release or the network path, and
    // quietly building from whatever happens to be on disk instead is not
    // the right answer to a possible tampering signal.
    if !verify_download(asset, &tmp) {
        fs::remove_file(&tmp).ok();
        std::process::exit(1);
    }
    println!("Checksum verified.");

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
        Command::new("powershell").args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script]).spawn().ok();
        println!("Downloaded the latest release. Finishing install in the background (re-run 'cforge' in a moment).");
    }
    true
}

fn update_from_source() {
    let src_dir = source_dir();
    if !src_dir.join("Cargo.toml").is_file() {
        eprintln!("Error: no local source checkout at {} either (set CFORGE_SOURCE to override).", src_dir.display());
        std::process::exit(1);
    }
    if !command_exists("cargo") {
        eprintln!("Error: cargo not found. Install Rust (https://rustup.rs) and re-run 'cforge self-update'.");
        std::process::exit(1);
    }

    println!("Building from local source at {}...", src_dir.display());
    let status = Command::new("cargo").args(["build", "--release"]).current_dir(&src_dir).status();
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

/// `--dry-run` promises to print actions instead of performing them. These
/// two commands overwrite and *delete* the installed binary, so ignoring the
/// flag here meant `cforge --dry-run self-uninstall` really uninstalled
/// cforge — the single worst place in the CLI to not honour it.
fn dry_run_notice(action: &str) -> bool {
    if crate::flags::get().dry_run {
        println!("+ {action}");
        true
    } else {
        false
    }
}

pub fn update_self() {
    if dry_run_notice(&format!("replace {} with the latest release", installed_path().display())) {
        return;
    }
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
        let cleaned: String =
            content.lines().filter(|l| *l != marker && *l != export_line).collect::<Vec<_>>().join("\n");
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
    if dry_run_notice(&format!("remove {} and its PATH entry", dest.display())) {
        return;
    }
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

#[cfg(test)]
mod tests {
    use super::parse_digest;

    #[test]
    fn accepts_a_bare_digest_and_the_sha256sum_form() {
        let d = "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0";
        assert_eq!(parse_digest(d).as_deref(), Some(d));
        assert_eq!(parse_digest(&format!("{d}\n")).as_deref(), Some(d));
        assert_eq!(parse_digest(&format!("{d}  cforge-linux-x86_64\n")).as_deref(), Some(d));
        // Windows' Get-FileHash emits uppercase.
        assert_eq!(parse_digest(&d.to_uppercase()).as_deref(), Some(d));
    }

    /// Verification is fail-closed, so anything that isn't unmistakably a
    /// digest has to come back None — an HTML error page served instead of
    /// the checksum file being the case that matters.
    #[test]
    fn rejects_non_digests() {
        assert_eq!(parse_digest(""), None);
        assert_eq!(parse_digest("   \n"), None);
        assert_eq!(parse_digest("<!DOCTYPE html><html>404</html>"), None);
        assert_eq!(parse_digest("deadbeef"), None, "too short");
        assert_eq!(parse_digest(&"a".repeat(65)), None, "too long");
        assert_eq!(parse_digest(&"z".repeat(64)), None, "not hex");
    }
}
