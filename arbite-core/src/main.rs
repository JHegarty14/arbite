use std::error::Error;

use arbite_core::{
    arbite_factory::ArbiteFactory, arbite_module::ArbiteModule, controllers::Controller,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>>
{
    println!("Hello, world!");
    let root_module = ArbiteModule {
        providers: vec![],
        imports: vec![],
        controllers: vec![Controller {
            base_path: "v1".to_string(),
            route_nodes: vec![],
        }],
    };
    let app = ArbiteFactory::create(&root_module);
    app.listen(3000).await
}
