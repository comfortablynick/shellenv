mod cli;
use anyhow::{Context, Result};
use clap::Clap;
use log::debug;
use serde::Deserialize;
use std::{
    default::Default,
    env,
    fmt::{self, Write as WriteFmt},
    fs::OpenOptions,
    io::{self, Read, Write},
    path::PathBuf,
    str::FromStr,
};

#[derive(Clap, Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
}

impl Shell {
    /// Get from last element of $SHELL.
    ///
    /// Example: SHELL=/usr/bin/zsh => Some(Shell::Zsh)
    pub fn from_env() -> Option<Self> {
        if let Ok(shell) = env::var("SHELL") {
            return Shell::from_str(
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
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "bash" => Ok(Self::Bash),
            "zsh" => Ok(Self::Zsh),
            "fish" => Ok(Self::Fish),
            _ => Err(std::io::Error::from(std::io::ErrorKind::NotFound)),
        }
    }
}

impl fmt::Display for Shell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Shell::Bash => write!(f, "bash"),
            Shell::Zsh => write!(f, "zsh"),
            Shell::Fish => write!(f, "fish"),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum VarType {
    Path,
    Env,
    Abbr,
    Alias,
}

#[derive(Deserialize, Debug)]
struct Config {
    path: Vec<Var>,
    env: Vec<Var>,
    abbr: Vec<Var>,
    alias: Vec<Var>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
/// Container for variable contents
struct Var {
    #[serde(skip_deserializing)]
    var_type: Option<VarType>,
    key: String,
    val: String,
    desc: Option<String>,
    args: Option<String>,
    cat: Option<String>,
    #[serde(default)]
    quote: bool,
    #[serde(default)]
    eval: bool,
    #[serde(default)]
    shell_eval: bool,
    #[serde(default = "default_shell")]
    shell: Vec<Shell>,
}

/// Shell value used when not present (all shells)
fn default_shell() -> Vec<Shell> {
    vec![Shell::Bash, Shell::Zsh, Shell::Fish]
}

/// Add escaped quotes if necessary
fn quote_if(quote: bool, s: &str) -> String {
    format!("{q}{}{q}", s, q = if quote { "\"" } else { "" })
}

impl fmt::Display for Var {
    /// Display based on POSIX format
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let quote_if = |quote: bool| if quote { "\"" } else { "" };
        match &self.var_type {
            Some(VarType::Path) => write!(
                f,
                "export PATH={q}{}{q}:$PATH",
                self.val.escape_debug(),
                q = quote_if(self.quote)
            ),
            Some(VarType::Env) => write!(
                f,
                "export {}={q}{}{q}",
                self.key,
                self.val.escape_debug(),
                q = quote_if(self.quote)
            ),
            Some(VarType::Abbr) => write!(
                f,
                "alias {}={q}{}{q}",
                self.key,
                self.val.escape_debug(),
                q = quote_if(self.quote)
            ),
            Some(VarType::Alias) => write!(
                f,
                "alias {}={q}{}{q}",
                self.key,
                self.val.escape_debug(),
                q = quote_if(self.quote)
            ),
            None => panic!("invalid var_type `{:?}`", self.var_type),
        }
    }
}

impl fmt::Debug for Var {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{}{}\n{:?}",
            if let Some(d) = &self.desc {
                format!("# {}\n", d)
            } else {
                String::new()
            },
            self,
            self.shell,
        )
    }
}

impl Var {
    #[allow(dead_code)]
    /// Quote val if Var.quote == true
    fn stringify_val(&self) -> String {
        if self.quote {
            return format!("{:?}", self.val);
        }
        self.val.clone()
    }

    /// Output in fish format
    fn to_fish_fmt(&self) -> String {
        match self.var_type {
            Some(VarType::Alias) => format!(
                "function {}; {} $argv; end; funcsave {}",
                self.key, self.val, self.key
            ),
            Some(VarType::Path) => format!(
                "set -g {} fish_user_paths {}",
                self.args.clone().unwrap_or_else(String::new),
                quote_if(self.quote, &self.val)
            ),
            Some(VarType::Env) => {
                format!("set -gx {} {}", self.key, quote_if(self.quote, &self.val))
            }
            Some(VarType::Abbr) => {
                format!("abbr -g {} {}", self.key, quote_if(self.quote, &self.val))
            }
            None => String::new(),
        }
    }

    /// Output in bash/zsh format
    fn to_posix_fmt(&self) -> String {
        format!("{}", self)
    }
}

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    env_logger::init();

    let file = (|| -> Result<_> {
        let mut buf = String::new();
        OpenOptions::new()
            .read(true)
            .open(&cli.toml_file)
            .with_context(|| format!("Could not find file {:?}", cli.toml_file))?
            .read_to_string(&mut buf)?;
        Ok(buf)
    })()?;
    let vals: Config = toml::from_str(&file)?;
    let mut vars: Vec<Var> = Default::default();

    for mut p in vals.path {
        p.var_type = Some(VarType::Path);
        vars.push(p);
    }
    for mut e in vals.env {
        e.var_type = Some(VarType::Env);
        vars.push(e);
    }
    for mut a in vals.abbr {
        a.var_type = Some(VarType::Abbr);
        vars.push(a);
    }
    for mut a in vals.alias {
        a.var_type = Some(VarType::Alias);
        vars.push(a);
    }

    let mut buf = String::new();
    for var in &vars {
        if var.shell.contains(&cli.shell) {
            match &cli.shell {
                Shell::Fish => writeln!(buf, "{}", var.to_fish_fmt())?,
                _ => writeln!(buf, "{}", var.to_posix_fmt())?,
            }
        }
    }
    io::stdout().write_all(buf.as_bytes())?;

    // Debug info
    debug!("{:#?}", cli);
    if log::max_level() == log::Level::Trace {
        for var in &vars {
            writeln!(buf, "{:?}", var)?;
        }
    }
    Ok(())
}
