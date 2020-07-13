mod cli;
mod config;
mod logger;
mod os;
mod shell;
mod util;
mod var;
use crate::{config::parse_config, util::file_to_string};
use clap::Clap;
use log::{info, trace};
use std::io;

type Result = anyhow::Result<()>;

/// Parse toml file and output shell rc file
fn main() -> Result {
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
    use crate::shell::Shell;

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
