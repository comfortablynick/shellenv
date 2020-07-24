//! Configure simple console logging with env_logger
use env_logger::{
    fmt::{Color, Style, StyledValue},
    Env,
};
use log::{self, Level};
use std::io::Write;

/// Initialize customized instance of env_logger
pub fn init_logger(verbose: u8) {
    env_logger::Builder::from_env(Env::new().default_filter_or(match verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    }))
    .format(|buf, record| {
        let mut style = buf.style();
        style.set_bold(true);
        let level = colored_level(&mut style, record.level());
        let mut style = buf.style();
        let target = style.set_bold(true).value(record.target());

        write!(buf, "{}|{}", level, target).unwrap();

        if let Some(file) = record.file() {
            write!(buf, "|{}", file).unwrap();
        }

        writeln!(buf, ": {}", record.args())
    })
    .init();
}

/// Style log level with color
fn colored_level(style: &mut Style, level: Level) -> StyledValue<String> {
    match level {
        Level::Trace => style.set_color(Color::Black),
        Level::Debug => style.set_color(Color::Cyan),
        Level::Info => style.set_color(Color::Green),
        Level::Warn => style.set_color(Color::Magenta),
        Level::Error => style.set_color(Color::Red),
    }
    .value(level.to_string())
}
