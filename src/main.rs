mod cli;
use anyhow::{Context, Result};
use clap::Clap;
use log::{debug, log_enabled, trace};
use serde::Deserialize;
use std::{
    borrow::Cow,
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
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "bash" | "sh" => Ok(Self::Bash),
            "zsh" => Ok(Self::Zsh),
            "fish" => Ok(Self::Fish),
            "pwsh" | "ps" | "powershell" => Ok(Self::Powershell),
            _ => Err(std::io::Error::from(std::io::ErrorKind::NotFound)),
        }
    }
}

impl fmt::Display for Shell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Shell::Bash => f.write_str("bash"),
            Shell::Zsh => f.write_str("zsh"),
            Shell::Fish => f.write_str("fish"),
            Shell::Powershell => f.write_str("powershell"),
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
struct Config<'a> {
    #[serde(borrow)]
    path: Vec<Var<'a>>,
    #[serde(borrow)]
    env: Vec<Var<'a>>,
    #[serde(borrow)]
    abbr: Vec<Var<'a>>,
    #[serde(borrow)]
    alias: Vec<Var<'a>>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
/// Container for variable contents
struct Var<'a> {
    #[serde(skip_deserializing)]
    var_type: Option<VarType>,
    key: &'a str,
    val: Cow<'a, str>,
    desc: Option<&'a str>,
    args: Option<&'a str>,
    cat: Option<&'a str>,
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
    vec![Shell::Bash, Shell::Zsh, Shell::Fish, Shell::Powershell]
}

/// Add escaped quotes if necessary
// fn quote_if(quote: bool, s: &str) -> String {
//     format!("{q}{}{q}", s, q = if quote { "\"" } else { "" })
// }

impl fmt::Display for Var<'_> {
    /// Display based on POSIX format
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(var_t) = &self.var_type {
            let val = if self.quote {
                format!("{:?}", self.val)
            } else {
                format!("{}", self.val)
            };
            match var_t {
                VarType::Path => write!(f, "export PATH={}:$PATH", val),
                VarType::Env => write!(f, "export {}={}", self.key, val),
                VarType::Abbr | VarType::Alias => write!(f, "alias {}={}", self.key, val),
            }
        } else {
            panic!("Invalid variable type: {:#?}", &self.var_type)
        }
    }
}

impl fmt::Debug for Var<'_> {
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

impl Var<'_> {
    /// Output in fish format
    fn to_fish_fmt(&self) -> String {
        if let Some(var_t) = &self.var_type {
            let val = if self.quote {
                format!("{:?}", self.val)
            } else {
                format!("{}", self.val)
            };
            match var_t {
                VarType::Alias => format!(
                    "function {}; {} $argv; end; funcsave {}",
                    self.key, self.val, self.key
                ),
                VarType::Path => format!(
                    "set -g {} fish_user_paths {}",
                    self.args.unwrap_or_default(),
                    val
                ),
                VarType::Env => format!("set -gx {} {}", self.key, val),
                VarType::Abbr => format!("abbr -g {} {}", self.key, val),
            }
        } else {
            panic!("Invalid variable type: {:#?}", &self.var_type);
        }
    }

    /// Output in powershell format
    fn to_powershell_fmt(&self) -> String {
        if let Some(var_t) = &self.var_type {
            match var_t {
                VarType::Alias | VarType::Abbr => {
                    let var = if self.quote {
                        format!("{:?}", self.val)
                    } else {
                        format!("{}", self.val)
                    };
                    format!("function {} {{ {} }}", self.key, var)
                }
                VarType::Path => format!("$Env:Path = {:?}", format!("{}:$Env:Path", self.val)),
                VarType::Env => format!("$Env:{} = {:?}", self.key, self.val),
            }
        } else {
            panic!("Invalid variable type: {:#?}", &self.var_type);
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

    let mut buf = String::with_capacity(4000);
    for var in &vars {
        if var.shell.contains(&cli.shell) {
            match &cli.shell {
                Shell::Fish => buf.push_str(&var.to_fish_fmt()),
                Shell::Powershell => buf.push_str(&var.to_powershell_fmt()),
                _ => buf.push_str(&var.to_posix_fmt()),
            }
            buf.push('\n');
        }
    }
    io::stdout().write_all(buf.as_bytes())?;

    // Debug info
    debug!("{:#?}", cli);
    if log_enabled!(log::Level::Trace) {
        let mut buf = String::new();
        for var in &vars {
            writeln!(buf, "{:?}", var)?;
        }
        trace!("\n{}", buf);
    }
    Ok(())
}
