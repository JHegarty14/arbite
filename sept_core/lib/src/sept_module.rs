use crate::graph::{Graph, Injected};
use actix_web::web::ServiceConfig;
use std::sync::Arc;
use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
};

pub trait ServiceFactory: Send + Sync {
    fn register(&self, app: &mut ServiceConfig);
}

pub(crate) struct ApplicationContext {
    pub(crate) global_providers: Graph,
    pub(crate) modules: HashMap<TypeId, Arc<ResolvedModule>>,
}
#[derive(Default)]
pub struct Module {
    exports: HashSet<TypeId>,
    imports: Vec<Box<dyn FnOnce(&mut ResolvedModule, &mut ApplicationContext)>>,
    providers: Vec<Box<dyn FnOnce(&mut ResolvedModule, &mut ApplicationContext)>>,
    clients: Vec<Box<dyn FnOnce(&mut ResolvedModule, &mut ApplicationContext)>>,
}

impl Module {
    pub fn new() -> Self {
        Self {
            exports: HashSet::new(),
            imports: Vec::new(),
            providers: Vec::new(),
            clients: Vec::new(),
        }
    }

    pub fn import<T: ModuleFactory + 'static>(mut self) -> Self {
        self.imports.push(Box::new(|module, ctx| {
            if let Some(resolved) = ctx.modules.get(&TypeId::of::<T>()) {
                module.imports.push(resolved.clone());
            } else {
                let new_module = Arc::new(T::get_module().build(ctx));
                ctx.modules.insert(TypeId::of::<T>(), new_module.clone());
                module.imports.push(new_module);
            }
        }));
        self
    }

    pub fn export<T>(mut self) -> Self
    where
        T: Injected + Send + Sync + 'static,
    {
        self.exports.insert(TypeId::of::<Arc<T>>());
        self
    }

    pub fn export_val<T>(mut self, _: &T) -> Self
    where
        T: Injected + Send + Sync + 'static,
    {
        self.exports.insert(TypeId::of::<T>());
        self
    }

    pub fn provide<T>(mut self) -> Self
    where
        T: Injected<Output = T> + 'static,
    {
        self.providers.push(Box::new(|module, ctx| {
            let mut graphs = vec![&ctx.global_providers];
            for module in &module.imports {
                graphs.push(&module.graphed_exports);
            }
        }));
        self
    }

    pub fn client<T>(mut self) -> Self
    where
        T: Injected<Output = T> + ServiceFactory + 'static,
    {
        self.clients.push(Box::new(|module, ctx| {
            let mut graphs = vec![&ctx.global_providers];
            for module in &module.imports {
                graphs.push(&module.graphed_exports);
            }
            let resolved = T::resolve(&mut module.graph, &graphs);
            module.clients.push(Arc::new(resolved));
        }));
        self
    }

    pub(crate) fn build(self, ctx: &mut ApplicationContext) -> ResolvedModule {
        let mut module = ResolvedModule::new();
        for import in self.imports {
            import(&mut module, ctx);
        }
        for provider in self.providers {
            provider(&mut module, ctx);
        }
        for client in self.clients {
            client(&mut module, ctx);
        }
        module.graphed_exports = module.graph.filter_by(self.exports);
        module
    }
}

pub trait ModuleFactory: Sized {
    fn get_module() -> Module;
}

#[derive(Clone)]
pub(crate) struct ResolvedModule {
    pub(crate) graph: Graph,
    pub(crate) imports: Vec<Arc<Self>>,
    graphed_exports: Graph,
    pub(crate) clients: Vec<Arc<dyn ServiceFactory>>,
}

impl ResolvedModule {
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
            imports: Vec::new(),
            graphed_exports: Graph::new(),
            clients: Vec::new(),
        }
    }
}
