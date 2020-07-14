use anyhow::anyhow;
use clap::Clap;
use log::{info, trace};
use shellenv::{
    config::parse_config,
    logger::Logger,
    shell::Shell,
    util::{file_to_string, Result},
};
use std::{io, path::PathBuf};

#[derive(Clap, Debug, PartialEq, Default)]
#[clap(author, about, version, max_term_width = 80)]
pub struct Cli {
    /// Location of toml file to parse (required)
    #[clap(
        name = "FILE",
        parse(from_os_str),
        env = "SHELLENV_FILE",
        hide_env_values = true
    )]
    pub toml_file: PathBuf,
    /// Shell to parse env for if not $SHELL
    #[clap(short, long, arg_enum, default_value)]
    pub shell:     Shell,
    ///Increase logging output to console
    #[clap(short, long = "verbose", parse(from_occurrences))]
    pub verbosity: u8,
}

/// Parse toml file and output shell rc file
fn main() -> Result {
    let cli = Cli::parse();
    Logger::init(cli.verbosity).map_err(|e| anyhow!(e))?;

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
    use shellenv::shell::Shell;
    use std::process::{Command, Stdio};

    const TEST_TOML: &str = "~/test.toml";

    #[test]
    fn simple() {
        let mut cli = Cli::default();
        cli.toml_file = PathBuf::from(TEST_TOML);
        cli.shell = Shell::Fish;
        assert_eq!(cli, Cli::parse_from(&["", TEST_TOML, "-s", "fish"]))
    }

    #[test]
    // TODO: get shell from env var
    fn shell_from_env() {
        let mut cli = Cli::default();
        cli.toml_file = PathBuf::from(TEST_TOML);
        cli.shell = Shell::Fish;
        assert_eq!(cli, Cli::parse_from(&["", TEST_TOML, "-s", "fish"]))
    }

    #[test]
    fn missing_toml_file() -> Result {
        let cmd = Command::new("shellenv")
            .arg("/tmp/noexist")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let result = cmd.wait_with_output()?;
        assert!(!result.status.success());
        Ok(())
    }

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
