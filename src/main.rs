#![allow(dead_code)]
#![allow(unused_imports)]
#![feature(option_flattening)]
use crate::cli::Cli;
use log::*;
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

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
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
                    .flatten()
                    .unwrap(),
            )
            .ok();
        }
        None
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
    path:  Vec<Var>,
    env:   Vec<Var>,
    abbr:  Vec<Var>,
    alias: Vec<Var>,
}

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
            Some(VarType::Alias) => format!("{}", self),
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
        // if let Some(var_type) = self.var_type {
        //     match var_type {
        //         VarType::Alias => format!("{}", self)
        //     }
        // }
        format!("{}", self)
    }
}

// Vars :: Newtype container for collection of Var {{{
// #[derive(Debug, Default)]
// struct Vars(Vec<Var>);
//
// impl fmt::Display for Vars {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{}", self)
//     }
// }
//
// impl IntoIterator for Vars {
//     type IntoIter = ::std::vec::IntoIter<Self::Item>;
//     type Item = Var;
//
//     fn into_iter(self) -> Self::IntoIter {
//         self.0.into_iter()
//     }
// } }}}

fn main() -> Result<(), std::io::Error> {
    let cli = cli::parse_args()?;
    env_logger::init();

    let mut buf = String::new();
    let _file = OpenOptions::new()
        .read(true)
        .open(&cli.toml_file)
        .unwrap_or_else(|_| panic!("cannot find file path: {:?}", cli.toml_file))
        .read_to_string(&mut buf)
        .unwrap();
    let vals: Config = toml::from_str(&buf).unwrap();
    // let mut vars: Vars = Default::default();
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
    if let Some(sh) = &cli.shell {
        for var in &vars {
            if var.shell.contains(sh) {
                match sh {
                    Shell::Fish => writeln!(buf, "{}", var.to_fish_fmt()).unwrap(),
                    _ => writeln!(buf, "{}", var.to_posix_fmt()).unwrap(),
                }
            }
        }
        io::stdout().write_all(buf.as_bytes())?;
    }
    // Debug info
    info!("{:#?}", cli);
    if log::max_level() == log::Level::Trace {
        for var in &vars {
            writeln!(buf, "{:?}", var).unwrap();
        }
    }
    Ok(())
}

mod cli {
    use crate::Shell;
    use clap::{
        app_from_crate, crate_authors, crate_description, crate_name, crate_version, value_t,
        values_t, AppSettings, ArgSettings,
    };
    use std::{
        path::{Path, PathBuf},
        str::FromStr,
    };

    type App = clap::App<'static>;
    type Arg = clap::Arg<'static>;

    #[derive(Debug, Default, Clone)]
    pub struct Cli {
        /// Location of toml file to parse (required)
        pub toml_file: PathBuf,
        /// Shell to parse env for (use current shell if not supplied)
        pub shell: Option<Shell>,
    }

    /// Parse cli arguments and return Cli struct with validated options
    pub fn parse_args() -> Result<Cli, std::io::Error> {
        let app = app_from_crate!()
            .setting(AppSettings::DontCollapseArgsInUsage)
            .setting(AppSettings::DeriveDisplayOrder)
            .setting(AppSettings::AllArgsOverrideSelf)
            .max_term_width(80)
            .after_help(
                "\
Environment variables:
    SHELLENV_FILE
            Used for <file>",
            )
            .arg(
                Arg::with_name("shell")
                    .settings(&[ArgSettings::MultipleValues, ArgSettings::HidePossibleValues])
                    .long("shell")
                    .short('s')
                    .help("Process environment for shell(s)")
                    .long_help(
                        "\
Process environment for shell(s). Output will be in file with the proper extension for the shell.

Possible values are \"bash\", \"zsh\", \"fish\".",
                    )
                    .possible_values(&["bash", "zsh", "fish"]),
            )
            .arg(
                Arg::with_name("file")
                    .settings(&[
                        ArgSettings::TakesValue,
                        ArgSettings::Required,
                        ArgSettings::HideEnvValues,
                    ])
                    .help("TOML file to parse for environment")
                    .long_help(
                        "\
TOML file to parse for the environment.

The settings inside the toml file with dictate how the variables are processed for each shell.",
                    )
                    .env("SHELLENV_FILE"),
            )
            .get_matches();
        let mut cli: Cli = Default::default();

        // process cli values
        cli.toml_file = value_t!(app, "file", PathBuf).unwrap();
        cli.shell = match app.value_of("shell") {
            Some(s) => Shell::from_str(s).ok(),
            None => Shell::from_env(),
        };
        Ok(cli)
    }
}
