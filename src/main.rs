#![allow(dead_code)]
#![allow(unused_imports)]
use crate::cli::Cli;
use serde::Deserialize;
use std::{fmt, fs::OpenOptions, io::Read};

#[derive(Debug)]
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

#[derive(Deserialize, Debug)]
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
    shell: Option<Vec<String>>,
}

impl fmt::Display for Var {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let quote_if = |quote: bool| if quote { "\"" } else { "" };
        write!(
            f,
            "export {}={q}{}{q}",
            self.key,
            self.val.escape_debug(),
            q = quote_if(self.quote)
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
}

fn main() -> Result<(), std::io::Error> {
    let cli = cli::parse_args()?;

    let mut buf = String::new();
    let _file = OpenOptions::new()
        .read(true)
        .open(&cli.toml_file)
        .unwrap_or_else(|_| panic!("cannot find file path: {:?}", cli.toml_file))
        .read_to_string(&mut buf)
        .unwrap();

    let vals: Config = toml::from_str(&buf).unwrap();
    let mut vars: Vec<Var> = vec![];

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

    for var in vars {
        println!("{}", var);
    }
    Ok(())
}

mod cli {
    use clap::{
        app_from_crate, crate_authors, crate_description, crate_name, crate_version, value_t,
        values_t, AppSettings, ArgSettings,
    };
    type App = clap::App<'static>;
    type Arg = clap::Arg<'static>;

    #[derive(Debug, Default, Clone)]
    pub struct Cli {
        pub toml_file: std::path::PathBuf,
    }

    pub fn parse_args() -> Result<Cli, std::io::Error> {
        let app = app_from_crate!()
            .setting(AppSettings::DontCollapseArgsInUsage)
            .setting(AppSettings::VersionlessSubcommands)
            .setting(AppSettings::DeriveDisplayOrder)
            .setting(AppSettings::AllArgsOverrideSelf)
            .setting(AppSettings::UnifiedHelpMessage)
            .arg(
                Arg::with_name("file")
                    .settings(&[ArgSettings::TakesValue, ArgSettings::Required])
                    .env("SHELLENV_FILE"),
            )
            .get_matches();
        let mut cli: Cli = Default::default();

        cli.toml_file = value_t!(app, "file", std::path::PathBuf).unwrap();
        Ok(cli)
    }
}
