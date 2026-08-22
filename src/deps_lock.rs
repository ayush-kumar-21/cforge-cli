//! `deps.lock` — records exactly what was installed for each library, so a
//! teammate (or CI) reproduces the same dependency set instead of re-running
//! platform-specific package-name guessing every time.
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const LOCK_FILE: &str = "deps.lock";

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DepsLock {
    #[serde(default)]
    pub library: Vec<LockedLib>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LockedLib {
    pub name: String,
    pub package: String,
    pub platform: String,
}

impl DepsLock {
    pub fn load() -> DepsLock {
        fs::read_to_string(LOCK_FILE).ok().and_then(|s| toml::from_str(&s).ok()).unwrap_or_default()
    }

    pub fn save(&self) {
        let content = toml::to_string_pretty(self).unwrap_or_default();
        crate::platform::write_file(Path::new(LOCK_FILE), &content);
    }

    /// Record (or update) the resolved package name for a library on the
    /// current platform. Idempotent: re-adding the same name+platform
    /// replaces the existing entry instead of duplicating it.
    pub fn record(&mut self, name: &str, package: &str, platform: &str) {
        self.library.retain(|l| !(l.name == name && l.platform == platform));
        self.library.push(LockedLib {
            name: name.to_string(),
            package: package.to_string(),
            platform: platform.to_string(),
        });
        self.library
            .sort_by(|a, b| (a.name.as_str(), a.platform.as_str()).cmp(&(b.name.as_str(), b.platform.as_str())));
    }

    pub fn remove(&mut self, name: &str) {
        self.library.retain(|l| l.name != name);
    }

    pub fn find(&self, name: &str, platform: &str) -> Option<&LockedLib> {
        self.library.iter().find(|l| l.name == name && l.platform == platform)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_then_find_roundtrips() {
        let mut lock = DepsLock::default();
        lock.record("sqlite3", "libsqlite3-dev", "linux");
        assert_eq!(lock.find("sqlite3", "linux").unwrap().package, "libsqlite3-dev");
        assert!(lock.find("sqlite3", "macos").is_none());
    }

    #[test]
    fn record_replaces_existing_entry_for_same_lib_and_platform() {
        let mut lock = DepsLock::default();
        lock.record("curl", "libcurl-dev", "linux");
        lock.record("curl", "libcurl4-openssl-dev", "linux");
        let matches: Vec<_> = lock.library.iter().filter(|l| l.name == "curl" && l.platform == "linux").collect();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].package, "libcurl4-openssl-dev");
    }

    #[test]
    fn remove_drops_all_platform_entries_for_a_library() {
        let mut lock = DepsLock::default();
        lock.record("zlib", "zlib1g-dev", "linux");
        lock.record("zlib", "zlib", "macos");
        lock.remove("zlib");
        assert!(lock.find("zlib", "linux").is_none());
        assert!(lock.find("zlib", "macos").is_none());
    }
}
