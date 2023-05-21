#[derive(Clone)]
pub struct AsyncInjector {
    inner: Arc<Inner>,
}

impl AsyncInjector {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for AsyncInjector {
    fn default() -> Self {
        Self {
            inner: Arc::new(Inner {
                map: HashMap::new(),
            }),
        }
    }
}