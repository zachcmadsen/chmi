use log::{Level, LevelFilter, Log, SetLoggerError};

pub struct Logger {
    verbose: bool,
}

impl Logger {
    pub fn init(verbose: bool) -> Result<(), SetLoggerError> {
        log::set_max_level(LevelFilter::Debug);
        log::set_boxed_logger(Box::new(Logger { verbose }))
    }
}

impl Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        let max_verbosity_level =
            if self.verbose { Level::Debug } else { Level::Info };
        metadata.level() <= max_verbosity_level
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        eprintln!("{}", record.args());
    }

    fn flush(&self) {}
}
