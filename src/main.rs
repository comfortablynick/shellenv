mod cli;
mod logger;
use anyhow::{Context, Result};
use clap::Clap;
use lazy_format::lazy_format;
use log::{debug, info};
use serde::Deserialize;
use std::{
    borrow::Cow,
    default::Default,
    env,
    fmt::{self, Debug, Display},
    fs::OpenOptions,
    io::{self, Read, Write},
    path::PathBuf,
    process::{Command, Output, Stdio},
    str::{self, FromStr},
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

#[derive(Debug, PartialEq, Eq)]
enum VarType {
    Path,
    Env,
    Abbr,
    Alias,
}

impl Default for VarType {
    fn default() -> Self {
        VarType::Env
    }
}

#[derive(Deserialize, Debug)]
struct Config<'a> {
    #[serde(borrow, default)]
    path:  Vec<Var<'a>>,
    #[serde(borrow, default)]
    env:   Vec<Var<'a>>,
    #[serde(borrow, default)]
    abbr:  Vec<Var<'a>>,
    #[serde(borrow, default)]
    alias: Vec<Var<'a>>,
}

impl Config<'_> {
    /// Get count of all vecs in the struct
    fn item_ct(&self) -> usize {
        &self.path.len() + &self.env.len() + &self.abbr.len() + &self.alias.len()
    }
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
/// Container for variable contents
struct Var<'a> {
    #[serde(skip_deserializing)]
    var_type:   VarType,
    #[serde(default)]
    key:        &'a str,
    val:        Cow<'a, str>,
    desc:       Option<&'a str>,
    args:       Option<&'a str>,
    cat:        Option<&'a str>,
    quote:      Option<bool>,
    #[serde(default)]
    eval:       bool,
    #[serde(default)]
    shell_eval: bool,
    #[serde(default = "default_shell")]
    shell:      Vec<Shell>,
}

/// Shell value used when not present (all shells)
fn default_shell() -> Vec<Shell> {
    vec![Shell::Bash, Shell::Zsh, Shell::Fish, Shell::Powershell]
}

/// Quote `s` if `quote` is true or if there are spaces
fn quote_if<'a, S>(s: S, quote: Option<bool>) -> Cow<'a, str>
where
    S: Into<Cow<'a, str>>,
{
    let do_quote = |x| Cow::Owned(format!("{:#?}", x));
    let s = s.into();
    match quote {
        Some(false) => s,
        Some(true) => do_quote(s),
        None => {
            if s.find(char::is_whitespace).is_some() {
                return do_quote(s);
            }
            s
        }
    }
}

impl Display for Var<'_> {
    /// Display based on POSIX format
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let val = if self.eval {
            let cmd: Vec<&str> = self.val.as_ref().split_whitespace().collect();
            let out = exec(&cmd).unwrap().stdout;
            Cow::from(str::from_utf8(&out).unwrap())
        } else {
            quote_if(self.val.as_ref(), self.quote)
        };
        write!(
            f,
            "{}",
            lazy_format!(match (self.var_type) {
                VarType::Path => ("export PATH={}:$PATH", val),
                VarType::Env => ("export {}={}", self.key, val),
                VarType::Abbr | VarType::Alias => ("alias {}={}", self.key, val),
            })
        )
    }
}

impl Var<'_> {
    /// Output in fish format
    fn to_fish_fmt(&self) -> String {
        let val = quote_if(self.val.as_ref(), self.quote);
        lazy_format!(match (self.var_type) {
            VarType::Alias => (
                "function {k}; {} $argv; end; funcsave {k}",
                self.val,
                k = self.key,
            ),
            VarType::Path => (
                "set -g {} fish_user_paths {}",
                self.args.unwrap_or_default(),
                val
            ),
            VarType::Env => ("set -gx {} {}", self.key, val),
            VarType::Abbr => ("abbr -g {} {}", self.key, val),
        })
        .to_string()
    }

    /// Output in powershell format
    fn to_powershell_fmt(&self) -> String {
        lazy_format!(match (self.var_type) {
            VarType::Alias | VarType::Abbr => ("function {} {{ {} }}", self.key, self.val),
            VarType::Path => ("$Env:Path = {:?}", format!("{}:$Env:Path", self.val)),
            VarType::Env => ("$Env:{} = {:?}", self.key, self.val),
        })
        .to_string()
    }

    /// Output in bash/zsh format
    fn to_posix_fmt(&self) -> String {
        format!("{}", self)
    }
}

/// Read file into string
fn file_to_string<P>(path: P) -> Result<String>
where
    P: Into<PathBuf> + Debug + Copy,
{
    let mut buf = String::new();
    OpenOptions::new()
        .read(true)
        .open(&path.into())
        .with_context(|| format!("Could not find file {:?}", &path))?
        .read_to_string(&mut buf)?;
    Ok(buf)
}

/// Spawn subprocess for `cmd` and access stdout/stderr
/// Fails if process output != 0
fn exec(cmd: &[&str]) -> Result<Output> {
    let command = Command::new(&cmd[0])
        .args(cmd.get(1..).expect("missing args in cmd"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let result = command.wait_with_output()?;

    if !result.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            str::from_utf8(&result.stderr)
                .unwrap_or("cmd returned non-zero status")
                .trim_end(),
        ))
        .with_context(|| format!("Command {:?}", cmd));
    }
    Ok(result)
}

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    logger::init_logger(cli.verbosity);

    let file = file_to_string(&cli.toml_file)?;
    let vals: Config = toml::from_str(&file)?;
    let mut vars: Vec<Var> = Vec::with_capacity(vals.item_ct());

    for mut v in vals.path {
        v.var_type = VarType::Path;
        vars.push(v);
    }
    for mut v in vals.env {
        v.var_type = VarType::Env;
        vars.push(v);
    }
    for mut v in vals.abbr {
        v.var_type = VarType::Abbr;
        vars.push(v);
    }
    for mut v in vals.alias {
        v.var_type = VarType::Alias;
        vars.push(v);
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

    info!("{:#?}", cli);
    debug!("{:#?}", vars);
    Ok(())
}
