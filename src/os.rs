#[cfg(unix)]
pub const SHELL: [&str; 2] = ["sh", "-c"];

#[cfg(windows)]
pub const SHELL: [&str; 2] = ["cmd.exe", "/c"];
