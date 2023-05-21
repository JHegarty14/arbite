use crate::di::Graph;
use crate::instrumentation::InstrumentationOpts;
use crate::sept_module::{ApplicationContext, ModuleFactory, ResolvedModule};
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

pub struct SeptApplication {
    app_config: SeptConfig,
    instrumentation: Option<InstrumentationOpts>,
}

impl SeptApplication {
    /// Creates a new instance of a sept application with the provided config
    pub fn new(app_config: SeptConfig) -> Self {
        Self {
            app_config,
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

    pub async fn init<T: ModuleFactory>(mut self) -> io::Result<()> {
        let mut fd = ListenFd::from_env();
        let mut ctx: ApplicationContext = self.app_config.register_globals();
        let module = Arc::new(T::get_module().build(&mut ctx));
        let mut server = HttpServer::new(move || {
            ActixApp::new().configure(|cfg| Self::configure(module.clone(), cfg))
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
