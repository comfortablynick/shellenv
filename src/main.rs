use serde::Deserialize;
use std::fs::OpenOptions;
use std::io::Read;
use toml::Value;

#[derive(Deserialize, Debug)]
struct Config {
    path: Vec<Var>,
    env: Vec<Var>,
    abbr: Vec<Var>,
    alias: Vec<Var>,
}

#[derive(Deserialize, Debug)]
struct Var {
    key: Option<String>,
    val: Option<String>,
    desc: Option<String>,
    args: Option<String>,
    cat: Option<String>,
    quote: Option<bool>,
    eval: Option<bool>,
    shell_eval: Option<bool>,
    shell: Option<Vec<String>>,
}

fn main() {
    let mut buf = String::new();
    let _file = OpenOptions::new()
        .read(true)
        .open("/home/nick/git/shellenv/src/env.toml")
        .unwrap()
        .read_to_string(&mut buf)
        .unwrap();

    let value = buf.parse::<Value>().unwrap();
    println!("{:#?}", value);
    let vals: Config = toml::from_str(&buf).unwrap();
    println!("{:#?}", vals);
}
