//! `cforge install`/`package`: `cmake --install` and `cpack` wrappers.
use crate::config::Config;
use crate::platform;
use std::path::PathBuf;

const PACKAGE_FORMATS: &[(&str, &str)] =
    &[("tgz", "TGZ"), ("zip", "ZIP"), ("dmg", "DragNDrop"), ("deb", "DEB"), ("rpm", "RPM")];

fn build_dir() -> PathBuf {
    PathBuf::from(Config::load().build_dir())
}

fn require_build_dir() {
    if !build_dir().join("CMakeCache.txt").exists() {
        eprintln!("Error: no configured build; run 'cforge build' first.");
        std::process::exit(1);
    }
}

pub fn install(prefix: Option<&str>) {
    require_build_dir();
    let flags = crate::flags::get();
    let mut args = vec!["--install".to_string(), ".".to_string()];
    if let Some(p) = prefix {
        args.push("--prefix".to_string());
        args.push(p.to_string());
    }
    if let Some(profile) = flags.profile {
        args.push("--config".to_string());
        args.push(profile.cmake_value().to_string());
    }
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    platform::run_or_die_in(&build_dir(), "cmake", &arg_refs, "install failed");
    platform::status("Installed.");
}

pub fn package(format: Option<&str>) {
    let fmt = format.unwrap_or("tgz");
    let Some((_, generator)) = PACKAGE_FORMATS.iter().find(|(k, _)| *k == fmt) else {
        crate::usage_error(&format!(
            "invalid --format value '{fmt}' (expected one of: {})",
            PACKAGE_FORMATS.iter().map(|(k, _)| *k).collect::<Vec<_>>().join("|")
        ));
    };
    require_build_dir();

    let flags = crate::flags::get();
    let mut args = vec!["-G".to_string(), generator.to_string()];
    if let Some(profile) = flags.profile {
        args.push("-C".to_string());
        args.push(profile.cmake_value().to_string());
    }
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    platform::run_or_die_in(&build_dir(), "cpack", &arg_refs, "package failed");
    platform::status(&format!("Packaged as {fmt}."));
}

#[cfg(test)]
mod tests {
    use super::PACKAGE_FORMATS;

    #[test]
    fn known_formats_cover_the_reference_doc_list() {
        for f in ["tgz", "zip", "dmg", "deb", "rpm"] {
            assert!(PACKAGE_FORMATS.iter().any(|(k, _)| *k == f));
        }
    }
}
