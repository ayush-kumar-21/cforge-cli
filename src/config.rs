//! `.cforge.toml` configuration — allows flexible source directory layouts.
//! Default paths: C/, CPP/, Obj_C/, Obj_CPP/, Swift/ can be overridden per-project.
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const CONFIG_FILE: &str = ".cforge.toml";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    #[serde(default = "default_paths")]
    pub paths: Paths,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Paths {
    #[serde(default = "default_c_src")]
    pub c_src: String,
    #[serde(default = "default_cpp_src")]
    pub cpp_src: String,
    #[serde(default = "default_obj_c_src")]
    pub obj_c_src: String,
    #[serde(default = "default_obj_cpp_src")]
    pub obj_cpp_src: String,
    #[serde(default = "default_swift_src")]
    pub swift_src: String,
    #[serde(default = "default_headers")]
    pub headers: String,
    #[serde(default = "default_build")]
    pub build: String,
}

fn default_c_src() -> String { "C".to_string() }
fn default_cpp_src() -> String { "CPP".to_string() }
fn default_obj_c_src() -> String { "Obj_C".to_string() }
fn default_obj_cpp_src() -> String { "Obj_CPP".to_string() }
fn default_swift_src() -> String { "Swift".to_string() }
fn default_headers() -> String { "include".to_string() }
fn default_build() -> String { "build".to_string() }

fn default_paths() -> Paths {
    Paths {
        c_src: default_c_src(),
        cpp_src: default_cpp_src(),
        obj_c_src: default_obj_c_src(),
        obj_cpp_src: default_obj_cpp_src(),
        swift_src: default_swift_src(),
        headers: default_headers(),
        build: default_build(),
    }
}

impl Config {
    /// Load .cforge.toml if it exists, otherwise return defaults.
    pub fn load() -> Config {
        if Path::new(CONFIG_FILE).exists() {
            match fs::read_to_string(CONFIG_FILE) {
                Ok(content) => match toml::from_str(&content) {
                    Ok(cfg) => cfg,
                    Err(e) => {
                        eprintln!("Warning: failed to parse {}: {}", CONFIG_FILE, e);
                        Config::default()
                    }
                },
                Err(e) => {
                    eprintln!("Warning: failed to read {}: {}", CONFIG_FILE, e);
                    Config::default()
                }
            }
        } else {
            Config::default()
        }
    }

    /// Write .cforge.toml with current settings.
    pub fn save(&self) {
        let content = toml::to_string_pretty(self).unwrap_or_default();
        crate::platform::write_file(Path::new(CONFIG_FILE), &(content + "\n"));
    }

    /// Get the source directory for a language.
    pub fn src_dir(&self, lang: &str) -> &str {
        match lang {
            "c" => &self.paths.c_src,
            "cpp" => &self.paths.cpp_src,
            "obj_c" => &self.paths.obj_c_src,
            "obj_cpp" => &self.paths.obj_cpp_src,
            "swift" => &self.paths.swift_src,
            _ => "src",
        }
    }

    /// Get all source directories (for discovering all targets).
    pub fn all_src_dirs(&self) -> [&str; 5] {
        [
            &self.paths.c_src,
            &self.paths.cpp_src,
            &self.paths.obj_c_src,
            &self.paths.obj_cpp_src,
            &self.paths.swift_src,
        ]
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            paths: default_paths(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_standard_paths() {
        let cfg = Config::default();
        assert_eq!(cfg.src_dir("c"), "C");
        assert_eq!(cfg.src_dir("cpp"), "CPP");
        assert_eq!(cfg.src_dir("obj_c"), "Obj_C");
        assert_eq!(cfg.src_dir("swift"), "Swift");
    }

    #[test]
    fn custom_paths_can_be_set() {
        let mut cfg = Config::default();
        cfg.paths.c_src = "src".to_string();
        cfg.paths.cpp_src = "src".to_string();
        assert_eq!(cfg.src_dir("c"), "src");
        assert_eq!(cfg.src_dir("cpp"), "src");
    }
}
