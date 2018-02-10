use log;
use log::{Log, LogRecord, LogLevel, LogMetadata};
use std::str::FromStr;

pub struct SimpleLogger {
    log_level: LogLevel
}

const DEFAULT_LOG_LEVEL: LogLevel = LogLevel::Error;


/// Initialize the SimpleLogger using the 'init()' function by passing it an Option<&str>
/// that has 'None' or 'Some("log_level_str")', where 'log_level_str' is a &str with a valid
/// log level, in any case. The string will be parsed and if valid set as the log level.
///
/// # Example
/// ```
/// #[macro_use]
/// extern crate log;
///
/// extern crate simplog;
/// use simplog::simplog::SimpleLogger;
///
/// fn main() {
///     SimpleLogger::init(None);
///     info!("Hello World!");
/// }
///
/// ```
impl SimpleLogger {
    pub fn init(arg: Option<&str>) {
        let level = parse_log_level(arg);
        log::set_logger(|max_log_level| {
            max_log_level.set(level.to_log_level_filter());
            Box::new(SimpleLogger {
                log_level: level
            })
        }).unwrap();
        println!("Logging at level {}", level);
    }
}

fn parse_log_level(arg: Option<&str>) -> LogLevel {
    match arg {
        None => DEFAULT_LOG_LEVEL,
        Some(arg) => match LogLevel::from_str(arg) {
            Ok(ll) => ll,
            Err(_) => DEFAULT_LOG_LEVEL
        }
    }
}

impl Log for SimpleLogger {
    fn enabled(&self, metadata: &LogMetadata) -> bool {
        metadata.level() <= self.log_level
    }

    fn log(&self, record: &LogRecord) {
        if self.enabled(record.metadata()) {
            println!("{} - {}", record.level(), record.args());
        }
    }
}

#[cfg(test)]
mod test {
    use log::LogLevel;

    #[test]
    fn no_log_level_arg() {
        assert_eq!(super::parse_log_level(None), super::DEFAULT_LOG_LEVEL);
    }

    #[test]
    fn invalid_log_level_arg() {
        assert_eq!(super::parse_log_level(Some("garbage")), super::DEFAULT_LOG_LEVEL);
    }

    #[test]
    fn info_log_level_arg() {
        assert_eq!(super::parse_log_level(Some("INFO")), LogLevel::Info);
    }

    #[test]
    fn error_log_level_arg() {
        assert_eq!(super::parse_log_level(Some("ERROR")), LogLevel::Error);
    }

    #[test]
    fn debug_log_level_arg() {
        assert_eq!(super::parse_log_level(Some("DEBUG")), LogLevel::Debug);
    }
}