use crate::Shell;
use anyhow::Result;
use clap::{
    crate_authors, crate_description, crate_name, crate_version, value_t, AppSettings, ArgSettings,
};
use std::{path::PathBuf, str::FromStr};

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
pub fn parse_args() -> Result<Cli> {
    let app = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
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
    cli.toml_file = value_t!(app, "file", PathBuf)?;
    cli.shell = match app.value_of("shell") {
        Some(s) => Shell::from_str(s).ok(),
        None => Shell::from_env(),
    };
    Ok(cli)
}
