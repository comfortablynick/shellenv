use crate::os;
use anyhow::{anyhow, Context, Result};
use log::{debug, warn};
use std::{
    borrow::Cow,
    fs::OpenOptions,
    io::Read,
    path::Path,
    process::{Command, Output, Stdio},
    str,
};

/// Read file into string
pub fn file_to_string<P>(path: P) -> Result<String>
where
    P: AsRef<Path>,
{
    OpenOptions::new()
        .read(true)
        .open(&path)
        .with_context(|| format!("Could not find file {:?}", path.as_ref().display()))
        .and_then(|mut file| {
            let mut contents = String::new();
            file.read_to_string(&mut contents)
                .map_err(|e| anyhow!("{}", e))
                .map(|_| contents)
        })
}

/// Spawn subprocess for `cmd` and access stdout/stderr
pub fn exec<I, T>(command: I) -> Result<Output>
where
    I: IntoIterator<Item = T>,
    T: Into<String>,
{
    let mut cmd = command.into_iter().map(Into::into);
    let mut spawn = Command::new(cmd.next().expect("Command missing"));
    while let Some(arg) = cmd.next() {
        spawn.arg(arg);
    }
    let result = spawn
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?
        .wait_with_output()
        .with_context(|| format!("Command failed: [{:?}]", spawn))?;

    if !result.status.success() {
        warn!("Command failed: [{:?}]; Result: {:?}", spawn, result);
    } else {
        debug!("Command: [{:?}]; Result: {:?}", spawn, result);
    }
    Ok(result)
}

/// Evaluate `cmd_str` in default os shell
pub fn shell_eval<'a, S: AsRef<str>>(cmd_str: S) -> Result<Cow<'a, str>> {
    let mut shell_cmd = Vec::from(os::SHELL);
    shell_cmd.push(cmd_str.as_ref());
    let result = exec(shell_cmd)?;
    let out = str::from_utf8(&result.stdout)?.trim_end().to_string();
    Ok(Cow::from(out))
}

/// Quote `s` if `quote` is true or if there is whitespace in `s`
pub fn quote_if<'a, S>(s: S, quote: Option<bool>) -> Cow<'a, str>
where
    S: Into<Cow<'a, str>>,
{
    let do_quote = |x| Cow::Owned(format!("{:#?}", x));
    let s = s.into();
    match quote {
        Some(false) => s,
        Some(true) => do_quote(s),
        None => {
            if s.find(char::is_whitespace).is_some() {
                return do_quote(s);
            }
            s
        }
    }
}

