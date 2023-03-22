use crate::router::route::Endpoint;

pub struct Controller {
    pub base_path: String,
    pub route_nodes: Vec<Endpoint>,
}
