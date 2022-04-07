use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct DependencyCache(Mutex<StackMap>);
impl DependencyCache {
    pub fn new() -> Self {
        Self(Mutex::new(StackMap::new()))
    }

    pub async fn insert<T: Any + Send + Sync>(&self, key: &str, value: T) {
        self.0.lock().await.insert(key, value)
    }

    pub async fn get<T: Any + Send + Sync>(&self, key: &str) -> Option<Arc<T>> {
        self.0.lock().await.get(key)
    }

    pub async fn contains(&self, key: &str) -> bool {
        self.0.lock().await.contains(key)
    }
}

struct StackMap {
    map: HashMap<String, Arc<dyn Any + Send + Sync>>,
    stack: Vec<String>,
}
impl StackMap {
    pub fn new() -> Self {
        Self{
            map: HashMap::new(),
            stack: Vec::new(),
        }
    }

    pub fn insert<T: Any + Send + Sync>(&mut self, key: &str, value: T) {
        let retval = self.map.insert(String::from(key), Arc::new(value));
        if let Some(_) = &retval {
            self.stack.push(String::from(key));
        }
    }

    pub fn get<T: Any + Send + Sync>(&self, key: &str) -> Option<Arc<T>> {
        Arc::clone(self.map.get(key)?).downcast::<T>().ok()
    }

    pub fn contains(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    fn remove(&mut self, key: &str) {
        self.map.remove(key);
    }
}

impl Drop for StackMap {
    fn drop(&mut self) {
        for key in self.stack.clone().iter().rev() {
            self.remove(key);
        }
    }
}
