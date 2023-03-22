use crate::{arbite_application::ArbiteApplication, arbite_module::ArbiteModule};

pub struct ArbiteFactory {}

impl ArbiteFactory {
    pub fn create(root_module: &ArbiteModule) -> ArbiteApplication {
        ArbiteApplication::new(root_module)
    }
}
