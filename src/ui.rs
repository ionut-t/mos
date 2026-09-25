//! Consistent, colorized terminal output for CLI commands.

use std::fmt::Display;

use colored::Colorize;

/// A completed, positive outcome. e.g. "Linked module 'shell'."
pub fn success(msg: impl Display) {
    println!("{} {}", "✓".green().bold(), msg);
}

/// A failure. Printed to stderr.
pub fn error(msg: impl Display) {
    eprintln!("{} {}", "✗".red().bold(), msg);
}

/// Something the user should notice but that isn't fatal.
pub fn warn(msg: impl Display) {
    println!("{} {}", "⚠".yellow().bold(), msg);
}

/// A section header introducing the following output, e.g. "Linking module 'shell'...".
pub fn step(msg: impl Display) {
    println!("{}", msg.to_string().bold());
}

/// An indented, secondary line under a step.
pub fn item(msg: impl Display) {
    println!("  {}", msg);
}

/// An indented, de-emphasized line under a step.
pub fn detail(msg: impl Display) {
    println!("  {}", msg.to_string().dimmed());
}

/// A colored bullet for list entries, e.g. profile/host listings.
pub fn bullet(active: bool) -> String {
    if active {
        "●".green().to_string()
    } else {
        "○".dimmed().to_string()
    }
}
