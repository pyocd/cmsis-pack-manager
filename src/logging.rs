use log::{self, Log, LogLevelFilter, LogMetadata, LogRecord, SetLoggerError};
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

pub struct SimpleLogger<T: Write> {
    sink: Mutex<T>,
}

impl<T: Write> SimpleLogger<T> {
    pub fn new(sink: T) -> Self {
        SimpleLogger { sink: Mutex::new(sink) }
    }
}

impl<T: Write + Send + Sync> Log for SimpleLogger<T> {
    fn enabled(&self, _: &LogMetadata) -> bool {
        true
    }

    fn log(&self, record: &LogRecord) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let mut sink = self.sink.lock().unwrap();
        let _ = writeln!(sink, "{:<6} {}", record.level(), record.args());
    }
}

pub fn log_to_file<T: AsRef<Path>>(path: T, max_log_level: LogLevelFilter) -> io::Result<()> {
    let file = File::create(path)?;

    log_to(file, max_log_level).map_err(|err| io::Error::new(io::ErrorKind::Other, err))
}

pub fn log_to_stderr(max_log_level: LogLevelFilter) -> Result<(), SetLoggerError> {
    log_to(io::stderr(), max_log_level)
}

pub fn log_to<T: Write + Send + Sync + 'static>(
    sink: T,
    max_log_level: LogLevelFilter,
) -> Result<(), SetLoggerError> {
    log::set_logger(|log_max_log_level| {
        log_max_log_level.set(max_log_level);

        Box::new(SimpleLogger::new(sink))
    })
}
