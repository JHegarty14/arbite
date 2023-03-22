use crate::controllers::Controller;
use crate::router::route::Endpoint;
use hyper::body::Incoming;
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::Hasher,
    sync::Arc,
};

use super::route::Resource;

pub struct Router<B = Incoming> {
    pub routes: Arc<HashMap<u64, Endpoint<B>>>,
}

impl Router {
    pub fn build() -> RouteBuilder {
        RouteBuilder {
            route_hasher: DefaultHasher::new(),
            route_map: HashMap::new(),
        }
    }
}

pub struct RouteBuilder {
    route_hasher: DefaultHasher,
    route_map: HashMap<u64, Endpoint>,
}

impl RouteBuilder {
    fn register_controller(&mut self, controller: &Controller) {
        for route_node in &controller.route_nodes {
            self.register_route(route_node, &controller.base_path);
        }
    }

    pub fn register_controllers(&mut self, controllers: &Vec<Controller>) {
        for controller in controllers {
            self.register_controller(controller);
        }
    }

    fn register_route(&mut self, route_node: &Endpoint, base_path: &str) {
        let mut path = route_node.path.clone();
        path.push_str(base_path);
        let endpoint = Endpoint {
            method: route_node.method.clone(),
            path,
            handler: route_node.handler,
        };
        endpoint.hash(&mut self.route_hasher);
        self.route_map.insert(self.route_hasher.finish(), endpoint);
    }

    pub fn compile(self) -> Router {
        Router {
            routes: Arc::new(self.route_map),
        }
    }
}
