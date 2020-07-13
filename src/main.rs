#[cfg(unix)]
mod os {
    pub const SHELL: [&str; 2] = ["sh", "-c"];
}

#[cfg(windows)]
mod os {
    pub const SHELL: [&str; 2] = ["cmd.exe", "/c"];
}

mod cli;
mod logger;
use anyhow::{Context, Result};
use clap::Clap;
use lazy_format::lazy_format;
use log::{debug, info, trace, warn};
use serde::Deserialize;
use std::{
    borrow::Cow,
    default::Default,
    env,
    fmt::{self, Debug, Display},
    fs::OpenOptions,
    io::{self, Read},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    str::{self, FromStr},
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

impl<'a> Config<'a> {
    /// Create `Config` from toml
    fn from_toml(toml: &'a str) -> Result<Self> {
        let toml = toml.into();
        toml::from_str(toml).with_context(|| "Error converting str to toml")
    }

    /// Get count of all vecs in the struct
    fn item_ct(&self) -> usize {
        &self.path.len() + &self.env.len() + &self.abbr.len() + &self.alias.len()
    }
}

#[derive(Deserialize, Debug)]
/// Container for variable contents
struct Var<'a> {
    #[serde(skip_deserializing)]
    var_type: VarType,
    #[serde(default)]
    key:      &'a str,
    val:      Cow<'a, str>,
    desc:     Option<&'a str>,
    args:     Option<&'a str>,
    cat:      Option<&'a str>,
    quote:    Option<bool>,
    #[serde(default)]
    eval:     bool,
    shell:    Option<Vec<Shell>>,
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
            shell_eval(self.val.as_ref()).expect(&format!("Eval failed on {:?}", self.val.as_ref()))
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
        let val = if self.eval {
            shell_eval(self.val.as_ref()).expect(&format!("Eval failed on {:?}", self.val.as_ref()))
        } else {
            quote_if(self.val.as_ref(), self.quote)
        };
        lazy_format!(match (self.var_type) {
            VarType::Alias => (
                "function {k}; {} $argv; end; funcsave {k}",
                self.val,
                k = self.key,
            ),
            VarType::Path => (
                "set {} fish_user_paths {}",
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
    P: AsRef<Path>,
{
    let mut buf = String::new();
    OpenOptions::new()
        .read(true)
        .open(&path)
        .with_context(|| format!("Could not find file {:?}", path.as_ref().display()))?
        .read_to_string(&mut buf)?;
    Ok(buf)
}

/// Spawn subprocess for `cmd` and access stdout/stderr
/// Fails if process output != 0
fn exec<I, T>(command: I) -> Result<Output>
where
    I: IntoIterator<Item = T>,
    T: Into<String>,
{
    let mut cmd = command.into_iter().map(Into::into);
    let mut spawn = Command::new(cmd.next().expect("Command missing"));
    while let Some(arg) = cmd.next() {
        spawn.arg(arg);
    }
    let result = spawn
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?
        .wait_with_output()
        .with_context(|| format!("Command failed: [{:?}]", spawn))?;

    if !result.status.success() {
        warn!("Command failed: [{:?}]; Result: {:?}", spawn, result);
    } else {
        debug!("Command: [{:?}]; Result: {:?}", spawn, result);
    }
    Ok(result)
}

fn shell_eval<'a, S: AsRef<str>>(cmd_str: S) -> Result<Cow<'a, str>> {
    let mut shell_cmd = Vec::from(os::SHELL);
    shell_cmd.push(cmd_str.as_ref());
    let result = exec(shell_cmd)?;
    let out = str::from_utf8(&result.stdout)?.trim_end().to_string();
    Ok(Cow::from(out))
}

fn parse_config<W>(toml_str: &str, shell: &Shell, writer: &mut W) -> Result<()>
where
    W: io::Write,
{
    let vals = Config::from_toml(&toml_str)?;
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

    for var in &vars {
        if let Some(sh) = var.shell.clone() {
            // If a value for var.shell has been supplied, make sure
            // it includes the shell we're evaluating for
            // `None` assumes compatibility with any shell
            if !sh.contains(shell) {
                debug!("Skipping {:?}", var);
                continue;
            }
        }
        match shell {
            Shell::Fish => writeln!(writer, "{}", &var.to_fish_fmt().replace("$(", "("))?,
            Shell::Powershell => writeln!(writer, "{}", &var.to_powershell_fmt())?,
            _ => writeln!(writer, "{}", &var.to_posix_fmt())?,
        }
    }
    Ok(())
}

/// Parse toml file and output shell rc file
fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    logger::init_logger(cli.verbosity);

    let file = file_to_string(&cli.toml_file)?;
    let stdout = io::stdout();
    let mut writer = stdout.lock();
    let vars = parse_config(&file, &cli.shell, &mut writer)?;

    info!("{:#?}", cli);
    trace!("{:#?}", vars);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    type Result = anyhow::Result<()>;

    #[test]
    fn simple_env_var() -> Result {
        const TOML: &str = r#"
        [[env]]
        key = 'LANG'
        val = 'en_US.utf8'
        cat = 'system'
        desc = 'Locale setting'
        shell = ['bash']
            "#;
        let mut buf = Vec::new();
        let _ = parse_config(&TOML, &Shell::Bash, &mut buf)?;
        let output = String::from_utf8(buf)?;
        assert_eq!(output, "export LANG=en_US.utf8\n");
        Ok(())
    }
}
