use crate::{
    shell::Shell,
    util::{quote_if, shell_eval},
};
use lazy_format::lazy_format;
use serde::Deserialize;
use std::{
    borrow::Cow,
    default::Default,
    fmt::{self, Debug, Display},
    str,
};

#[derive(Debug, PartialEq, Eq)]
pub enum VarType {
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
/// Container for variable contents
pub struct Var<'a> {
    #[serde(skip_deserializing)]
    pub var_type: VarType,
    #[serde(default)]
    pub key:      &'a str,
    pub val:      Cow<'a, str>,
    pub desc:     Option<&'a str>,
    pub args:     Option<&'a str>,
    pub cat:      Option<&'a str>,
    pub quote:    Option<bool>,
    #[serde(default)]
    pub eval:     bool,
    pub shell:    Option<Vec<Shell>>,
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
    pub fn to_fish_fmt(&self) -> String {
        let mut val = if self.eval {
            shell_eval(self.val.as_ref()).expect(&format!("Eval failed on {:?}", self.val.as_ref()))
        } else {
            quote_if(self.val.as_ref(), self.quote)
        };
        if val.contains("$(") {
            val = val.replace("$(", "(").into();
        }
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
    pub fn to_powershell_fmt(&self) -> String {
        lazy_format!(match (self.var_type) {
            VarType::Alias | VarType::Abbr => ("function {} {{ {} }}", self.key, self.val),
            VarType::Path => ("$Env:Path = {:?}", format!("{}:$Env:Path", self.val)),
            VarType::Env => ("$Env:{} = {:?}", self.key, self.val),
        })
        .to_string()
    }

    /// Output in bash/zsh format
    pub fn to_posix_fmt(&self) -> String {
        format!("{}", self)
    }
}
