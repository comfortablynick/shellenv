use crate::Shell;
use clap::Clap;
use std::path::PathBuf;

#[derive(Clap, Debug)]
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
}
