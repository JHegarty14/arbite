use crate::graph::Graph;
use crate::instrumentation::InstrumentationOpts;
use crate::sept_module::{ApplicationContext, ModuleFactory, ResolvedModule};
use actix_cors::Cors;
use actix_tls::accept::rustls::reexports::ServerConfig;
use actix_web::web::ServiceConfig;
use actix_web::{App as ActixApp, HttpServer};
use listenfd::ListenFd;
use std::collections::HashMap;
use std::{io, sync::Arc};

pub struct SeptConfig {
    pub port: u16,
    pub tls_config: Option<ServerConfig>,
}

impl SeptConfig {
    fn new() -> Self {
        SeptConfig {
            port: 3000,
            tls_config: None,
        }
    }

    fn register_globals(&mut self) -> ApplicationContext {
        ApplicationContext {
            global_providers: Graph::new(),
            modules: HashMap::new(),
        }
    }
}

#[derive(Clone)]
pub struct CorsConfig {
    pub allowed_origin: String,
    pub allowed_methods: Vec<actix_http::Method>,
    pub allowed_headers: Vec<actix_http::header::HeaderName>,
    pub allow_credentials: bool,
    pub max_age: Option<usize>,
    pub expose_headers: Vec<String>,
}

pub struct SeptApplication {
    app_config: SeptConfig,
    cors: CorsConfig,
    instrumentation: Option<InstrumentationOpts>,
}

impl SeptApplication {
    /// Creates a new instance of a sept application with the provided config
    pub fn new(app_config: SeptConfig) -> Self {
        Self {
            app_config,
            cors: CorsConfig::default(),
            instrumentation: None,
        }
    }

    fn configure(module: Arc<ResolvedModule>, config: &mut ServiceConfig) {
        for client in &module.clients {
            client.register(config);
        }
        for imports in &module.imports {
            Self::configure(imports.clone(), config);
        }
    }

    /// Method to enable default instrumentation for the application
    pub fn instrument(mut self) -> Self {
        self.instrumentation = Some(InstrumentationOpts::default());
        self
    }

    //// Method to enable tracing with a custom log provider and subscriber
    pub fn with_instrumentation(mut self, opts: InstrumentationOpts) -> Self {
        self.instrumentation = Some(opts);
        self
    }

    pub fn with_cors(mut self, cors_opts: CorsConfig) -> Self {
        self.cors = cors_opts;
        self
    }

    pub async fn init<T: ModuleFactory>(mut self) -> io::Result<()> {
        let mut fd = ListenFd::from_env();
        let mut ctx: ApplicationContext = self.app_config.register_globals();
        let module = Arc::new(T::get_module().build(&mut ctx));
        let mut server = HttpServer::new(move || {
            let cors_config = self.cors.clone();
            let cors = Cors::default()
                .allowed_origin(&cors_config.allowed_origin)
                .allowed_methods(cors_config.allowed_methods)
                .allowed_headers(cors_config.allowed_headers)
                .expose_headers(cors_config.expose_headers)
                .max_age(cors_config.max_age);

            ActixApp::new().wrap(cors).configure(|cfg| Self::configure(module.clone(), cfg))
        });

        if cfg!(feature = "rustls") && self.app_config.tls_config.is_some() {
            server = match fd.take_tcp_listener(0).unwrap() {
                Some(listener) => {
                    server.listen_rustls(listener, self.app_config.tls_config.unwrap())?
                }
                None => server.bind_rustls(
                    format!("0.0.0.0:{}", self.app_config.port),
                    self.app_config.tls_config.unwrap(),
                )?,
            }
        } else {
            server = match fd.take_tcp_listener(0).unwrap() {
                Some(listener) => server.listen(listener)?,
                None => server.bind(format!("0.0.0.0:{}", self.app_config.port))?,
            };
        }

        server.run().await
    }
}

impl Default for SeptApplication {
    fn default() -> Self {
        Self::new(SeptConfig::new())
    }
}

impl Default for CorsConfig {
    fn default() -> CorsConfig {
        CorsConfig {
            allowed_origin: "http://localhost:3000".to_string(),
            allowed_methods: vec![actix_http::Method::GET, actix_http::Method::POST, actix_http::Method::PUT, actix_http::Method::DELETE],
            allowed_headers: vec![
                actix_web::http::header::AUTHORIZATION,
                actix_web::http::header::ACCEPT,
                actix_web::http::header::CONTENT_TYPE,
            ],
            allow_credentials: true,
            max_age: None,
            expose_headers: vec![],
        }
    }
}