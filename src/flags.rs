use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Profile {
    Debug,
    Release,
    RelWithDebInfo,
}

impl Profile {
    pub fn parse(s: &str) -> Option<Profile> {
        match s {
            "debug" => Some(Profile::Debug),
            "release" => Some(Profile::Release),
            "relwithdebinfo" => Some(Profile::RelWithDebInfo),
            _ => None,
        }
    }

    pub fn cmake_value(&self) -> &'static str {
        match self {
            Profile::Debug => "Debug",
            Profile::Release => "Release",
            Profile::RelWithDebInfo => "RelWithDebInfo",
        }
    }
}

#[derive(Clone, Default, Debug)]
pub struct Flags {
    pub help: bool,
    pub version: bool,
    pub verbose: u8,
    pub quiet: bool,
    pub profile: Option<Profile>,
    pub jobs: Option<u32>,
    pub no_color: bool,
    pub dry_run: bool,
}

/// Pulls every recognized global flag out of `args` regardless of where it
/// appears, returning the flags plus whatever tokens remain (the command
/// word and its own arguments), in their original relative order.
pub fn extract(args: &[String]) -> (Flags, Vec<String>) {
    let mut f = Flags::default();
    let mut rest = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => f.help = true,
            "-V" | "--version" => f.version = true,
            "-v" | "--verbose" => f.verbose = f.verbose.saturating_add(1),
            "-vv" => f.verbose = f.verbose.saturating_add(2),
            "-q" | "--quiet" => f.quiet = true,
            "--no-color" => f.no_color = true,
            "--dry-run" => f.dry_run = true,
            "--profile" => {
                i += 1;
                let val = args.get(i).map(|s| s.as_str()).unwrap_or("");
                f.profile = Some(Profile::parse(val).unwrap_or_else(|| {
                    crate::usage_error(&format!(
                        "invalid --profile value '{val}' (expected debug|release|relwithdebinfo)"
                    ))
                }));
            }
            "--jobs" => {
                i += 1;
                let val = args.get(i).map(|s| s.as_str()).unwrap_or("");
                f.jobs = Some(val.parse().unwrap_or_else(|_| {
                    crate::usage_error(&format!("invalid --jobs value '{val}' (expected a positive integer)"))
                }));
            }
            other => rest.push(other.to_string()),
        }
        i += 1;
    }
    (f, rest)
}

static FLAGS: OnceLock<Flags> = OnceLock::new();

pub fn init(f: Flags) {
    FLAGS.set(f).ok();
}

pub fn get() -> Flags {
    FLAGS.get().cloned().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(strs: &[&str]) -> Vec<String> {
        strs.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn flags_parse_regardless_of_position() {
        let (f1, rest1) = extract(&s(&["-v", "--jobs", "4", "build", "foo"]));
        let (f2, rest2) = extract(&s(&["build", "foo", "-v", "--jobs", "4"]));
        assert_eq!(f1.verbose, f2.verbose);
        assert_eq!(f1.jobs, f2.jobs);
        assert_eq!(rest1, s(&["build", "foo"]));
        assert_eq!(rest2, s(&["build", "foo"]));
    }

    #[test]
    fn vv_counts_as_two() {
        let (f, _) = extract(&s(&["-vv", "build"]));
        assert_eq!(f.verbose, 2);
        let (f, _) = extract(&s(&["-v", "-v", "build"]));
        assert_eq!(f.verbose, 2);
    }

    #[test]
    fn profile_parses_known_values() {
        let (f, _) = extract(&s(&["--profile", "release", "build"]));
        assert_eq!(f.profile, Some(Profile::Release));
    }

    #[test]
    fn get_defaults_when_uninitialized() {
        // flags::init() is never called in this test binary's unit-test
        // pass, so get() must not panic.
        let f = get();
        assert!(!f.quiet);
    }
}
