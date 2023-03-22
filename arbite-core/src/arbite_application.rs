use bytes::Bytes;
use http_body_util::Full;
use hyper::server::conn::http1;
use hyper::service::Service;
use hyper::Error;
use hyper::{body::Incoming as IncomingBody, Request, Response};
use tokio::net::TcpListener;

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::future::Future;
use std::hash::Hasher;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;

use crate::arbite_module::ArbiteModule;
use crate::router::route::{Endpoint, Resource, RouteMatcher};
use crate::router::router::Router;

pub struct ArbiteApplication {
    router: Router,
}

impl ArbiteApplication {
    pub fn new(root_module: &ArbiteModule) -> ArbiteApplication {
        let mut route_builder = Router::build();
        route_builder.register_controllers(&root_module.controllers);

        ArbiteApplication {
            router: route_builder.compile(),
        }
    }

    pub async fn listen(&self, port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let addr: SocketAddr = ([127, 0, 0, 1], port).into();

        let listener = TcpListener::bind(addr).await?;
        println!("Listening on http://{}", addr);

        loop {
            let (stream, _) = listener.accept().await?;
            let routes = Arc::clone(&self.router.routes);

            tokio::task::spawn(async move {
                if let Err(err) = http1::Builder::new()
                    .serve_connection(stream, RouterSvc { route_map: routes })
                    .await
                {
                    println!("Failed to serve connection: {:?}", err);
                }
            });
        }
    }
}

struct RouterSvc {
    route_map: Arc<HashMap<u64, Endpoint>>,
}

impl Service<Request<IncomingBody>> for RouterSvc {
    type Response = Response<Full<Bytes>>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&mut self, req: Request<IncomingBody>) -> Self::Future {
        let mut route_hasher = DefaultHasher::new();
        let route_matcher = RouteMatcher {
            method: req.method().clone(),
            path: String::from(req.uri().path()),
        };
        route_matcher.hash(&mut route_hasher);
        let route_hash = route_hasher.finish();
        let endpoint = match self.route_map.get(&route_hash) {
            Some(e) => e,
            None => panic!("Route not found"),
        };
        let handler = endpoint.handler;
        let response = handler(&req);
        Box::pin(async move { Ok(response) })
    }
}
