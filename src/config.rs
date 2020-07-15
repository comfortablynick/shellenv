use crate::{
    shell::Shell,
    util::Result,
    var::{Var, VarType},
};
use anyhow::Context;
use log::debug;
use serde::Deserialize;
use std::io;

#[derive(Deserialize, Debug)]
pub struct Config<'a> {
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

pub fn parse_config<W>(toml_str: &str, shell: &Shell, writer: &mut W) -> Result<()>
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
        writeln!(
            writer,
            "{}",
            match shell {
                Shell::Fish => var.to_fish_fmt(),
                Shell::Powershell => var.to_powershell_fmt(),
                _ => var.to_posix_fmt(),
            }
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toml_to_env_var() -> Result {
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
