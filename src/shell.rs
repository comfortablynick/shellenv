//! Enumerate shells and shell behavior
use clap::Clap;
use serde::Deserialize;
use std::{
    env,
    fmt::{self, Display},
    io,
    path::PathBuf,
    str::FromStr,
};

#[derive(Clap, Debug, Clone, Deserialize, PartialEq, Eq, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Shell {
    #[clap(alias = "sh")]
    Bash,
    Zsh,
    Fish,
    #[clap(alias = "ps", alias = "pwsh")]
    Powershell,
}

impl Shell {
    /// Get from last element of $SHELL.
    ///
    /// Example: SHELL=/usr/bin/zsh => Some(Shell::Zsh)
    pub fn from_env() -> Option<Self> {
        if let Ok(shell) = env::var("SHELL") {
            return Self::from_str(
                PathBuf::from(shell)
                    .file_name()
                    .map(|s| s.to_str())
                    .flatten()?,
            )
            .ok();
        }
        None
    }
}

impl Default for Shell {
    fn default() -> Self {
        Shell::from_env().expect("Could not determine shell.")
    }
}

impl FromStr for Shell {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, io::Error> {
        match s {
            "bash" | "sh" => Ok(Self::Bash),
            "zsh" => Ok(Self::Zsh),
            "fish" => Ok(Self::Fish),
            "pwsh" | "ps" | "powershell" => Ok(Self::Powershell),
            _ => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Cannot convert {:?} to Shell", s),
            )),
        }
    }
}

impl Display for Shell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Shell::Bash => f.write_str("bash"),
            Shell::Zsh => f.write_str("zsh"),
            Shell::Fish => f.write_str("fish"),
            Shell::Powershell => f.write_str("powershell"),
        }
    }
}
