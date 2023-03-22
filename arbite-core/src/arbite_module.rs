use crate::controllers::Controller;

pub struct ArbiteModule {
    pub providers: Vec<String>,
    pub imports: Vec<ArbiteModule>,
    pub controllers: Vec<Controller>,
}
