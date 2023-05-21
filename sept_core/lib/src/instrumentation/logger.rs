use crate::di::Graph;

use super::{InstrumentationProvider, LogLevel};
use futures_util::future::{ok, Ready};
use std::sync::Arc;

#[derive(Clone)]
pub struct Logger {
    logging_provider: Arc<dyn InstrumentationProvider>,
    log_level: LogLevel,
}

impl Logger {
    pub fn new(logging_provider: Arc<dyn InstrumentationProvider>, log_level: LogLevel) -> Self {
        Self {
            logging_provider,
            log_level,
        }
    }

    #[inline]
    pub fn debug(&self, message: String) {
        if self.log_level as i32 <= LogLevel::Debug as i32 {
            self.logging_provider.debug(message);
        }
    }

    #[inline]
    pub fn info(&self, message: String) {
        if self.log_level as i32 <= LogLevel::Info as i32 {
            self.logging_provider.info(message);
        }
    }

    #[inline]
    pub fn warn(&self, message: String) {
        if self.log_level as i32 <= LogLevel::Warn as i32 {
            self.logging_provider.warn(message);
        }
    }

    #[inline]
    pub fn error(&self, message: String) {
        if self.log_level as i32 <= LogLevel::Error as i32 {
            self.logging_provider.error(message);
        }
    }

    #[inline]
    pub fn trace(&self, message: String) {
        if self.log_level as i32 <= LogLevel::Trace as i32 {
            self.logging_provider.trace(message);
        }
    }
}

impl crate::di::Injected for Logger {
    type Output = Self;
    fn resolve<'a>(_: &'a mut crate::di::Graph, _: &[&Graph]) -> Self {
        panic!("Logger not configured in application!")
    }
}

impl actix_web::FromRequest for Logger {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    #[inline]
    fn from_request(req: &actix_web::HttpRequest, _: &mut actix_http::Payload) -> Self::Future {
        match req.app_data::<actix_web::web::Data<Self>>() {
            Some(s) => ok(s.get_ref().clone()),
            None => panic!("Logger not configured in application!"),
        }
    }
}
