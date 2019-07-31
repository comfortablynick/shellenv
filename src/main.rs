#![allow(dead_code)]
use serde::Deserialize;
use std::fs::OpenOptions;
use std::io::Read;

#[derive(Deserialize, Debug)]
struct Config {
    path: Vec<Var>,
    env: Vec<Var>,
    abbr: Vec<Var>,
    alias: Vec<Var>,
}

#[derive(Deserialize, Debug)]
struct Var {
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

impl Var {
    /// Quote val if Var.quote == true
    fn stringify_val(&self) -> String {
        if self.quote {
            return format!("{:?}", self.val);
        }
        self.val.clone()
    }
}

fn main() {
    let mut buf = String::new();
    let _file = OpenOptions::new()
        .read(true)
        .open("/home/nick/git/shellenv/src/env.toml")
        .unwrap()
        .read_to_string(&mut buf)
        .unwrap();

    let vals: Config = toml::from_str(&buf).unwrap();
    let quote_if = |quote: bool| if quote { "\"" } else { "" };
    for p in vals.path {
        println!("export PATH={}:$PATH", p.val);
    }
    for e in vals.env {
        println!(
            "export {}={q}{}{q}",
            e.key,
            e.val.escape_debug(),
            q = quote_if(e.quote)
        );
    }
}
