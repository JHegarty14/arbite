use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};
use std::hash::{Hash, Hasher};

pub trait Resource {
    fn hash<H: Hasher>(&self, state: &mut H) -> ();
}

pub struct Endpoint<B = Incoming, R = Full<Bytes>> {
    pub method: Method,
    pub path: String,
    pub handler: fn(&Request<B>) -> Response<R>,
}

impl Resource for Endpoint {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.method.hash(state);
        self.path.hash(state);
    }
}

pub struct RouteMatcher {
    pub method: Method,
    pub path: String,
}

impl Resource for RouteMatcher {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.method.hash(state);
        self.path.hash(state);
    }
}

macro_rules! http_method_type {
    ($($method:ident),*) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum HttpMethodType {
            $($method),*
        }

        impl HttpMethodType {
            pub fn as_str(&self) -> &'static str {
                match *self {
                    $(HttpMethodType::$method => stringify!($method)),*
                }
            }

            pub fn parse(s: &str) -> Result<HttpMethodType, String> {
                match s {
                    $(stringify!($method) => Ok(HttpMethodType::$method)),*,
                    _ => Err(format!("Unrecognized HTTP method: {}", s)),
                }
            }
        }
    }
}

http_method_type! {
    Get,
    Post,
    Put,
    Delete,
    Head,
    Connect,
    Options,
    Trace,
    Patch
}
