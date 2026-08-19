//! Minimal ANSI coloring. Disabled by `--no-color`, `NO_COLOR` (any value,
//! per no-color.org), or when stdout isn't a terminal — never emitted into
//! a redirected/piped log.
use std::io::IsTerminal;

fn enabled() -> bool {
    if crate::flags::get().no_color || std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    std::io::stdout().is_terminal()
}

fn wrap(code: &str, s: &str) -> String {
    if enabled() {
        format!("\x1b[{code}m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

pub fn red(s: &str) -> String {
    wrap("31", s)
}
pub fn green(s: &str) -> String {
    wrap("32", s)
}
pub fn yellow(s: &str) -> String {
    wrap("33", s)
}
