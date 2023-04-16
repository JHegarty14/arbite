pub mod logger;

pub struct InstrumentationOpts {
    pub level: LogLevel,
    pub provider: Box<dyn InstrumentationProvider>,
}

impl Default for InstrumentationOpts {
    fn default() -> Self {
        Self {
            level: LogLevel::Debug,
            provider: Box::new(NoopInstrumentationProvider),
        }
    }
}

#[derive(Clone, Copy)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

pub trait InstrumentationProvider: Sync + Send {
    fn debug(&self, message: String);

    fn info(&self, message: String);

    fn warn(&self, message: String);

    fn error(&self, message: String);

    fn trace(&self, message: String);
}

#[derive(Clone)]
pub struct NoopInstrumentationProvider;

impl NoopInstrumentationProvider {
    #[inline]
    fn log(log_level: LogLevel, message: String) {
        match log_level {
            LogLevel::Error => eprintln!("{}", message),
            LogLevel::Warn => eprintln!("{}", message),
            LogLevel::Info => eprintln!("{}", message),
            LogLevel::Debug => eprintln!("{}", message),
            LogLevel::Trace => eprintln!("{}", message),
        }
    }
}

impl InstrumentationProvider for NoopInstrumentationProvider {
    fn debug(&self, message: String) {
        Self::log(LogLevel::Debug, message);
    }

    fn info(&self, message: String) {
        Self::log(LogLevel::Info, message);
    }

    fn warn(&self, message: String) {
        Self::log(LogLevel::Warn, message);
    }

    fn error(&self, message: String) {
        Self::log(LogLevel::Error, message);
    }

    fn trace(&self, message: String) {
        Self::log(LogLevel::Trace, message);
    }
}
