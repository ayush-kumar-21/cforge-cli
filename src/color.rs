//! Minimal ANSI coloring. Disabled by `--no-color`, `NO_COLOR` (any value,
//! per no-color.org), or when the destination stream isn't a terminal —
//! never emitted into a redirected/piped log.
use std::io::IsTerminal;

fn wrap(code: &str, s: &str, stream_is_terminal: bool) -> String {
    if crate::flags::get().no_color || std::env::var_os("NO_COLOR").is_some() || !stream_is_terminal {
        return s.to_string();
    }
    format!("\x1b[{code}m{s}\x1b[0m")
}

fn stdout_tty() -> bool {
    std::io::stdout().is_terminal()
}

pub fn red(s: &str) -> String {
    wrap("31", s, stdout_tty())
}
pub fn green(s: &str) -> String {
    wrap("32", s, stdout_tty())
}
pub fn yellow(s: &str) -> String {
    wrap("33", s, stdout_tty())
}

/// Red for text going to stderr. Gating stderr output on whether *stdout*
/// is a terminal is simply the wrong stream: `cforge build 2>err.log` wrote
/// escape codes into the log whenever stdout stayed a tty, and
/// `cforge build >out.log` stripped color from errors still on screen.
pub fn red_err(s: &str) -> String {
    wrap("31", s, std::io::stderr().is_terminal())
}
