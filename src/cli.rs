use crate::Shell;
use clap::Clap;
use std::path::PathBuf;

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
    pub shell: Shell,
    ///Increase logging output to console
    #[clap(short, long, parse(from_occurrences))]
    pub verbose: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_TOML: &str = "~/test.toml";

    #[test]
    fn simple() {
        let mut cli = Cli::default();
        cli.toml_file = PathBuf::from(TEST_TOML);
        cli.shell = Shell::Fish;
        assert_eq!(cli, Cli::parse_from(&["", TEST_TOML, "-s", "fish"]))
    }

    #[test]
    fn shell_from_env() {
        let mut cli = Cli::default();
        cli.toml_file = PathBuf::from(TEST_TOML);
        cli.shell = Shell::Fish;
        assert_eq!(cli, Cli::parse_from(&["", TEST_TOML, "-s", "fish"]))
    }
}
