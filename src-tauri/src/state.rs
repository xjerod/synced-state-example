use std::{
    any::Any,
    collections::HashMap,
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::Event;

#[derive(Deserialize, Serialize, Type, Clone, Debug, Event)]
pub struct Update {
    pub version: Option<u128>,
    pub name: String,
    pub value: String,
}

pub struct StateSyncer {
    data: Mutex<HashMap<String, Box<Arc<dyn Any>>>>,
}

impl StateSyncer {
    fn new() -> Self {
        StateSyncer {
            data: Mutex::new(HashMap::new()),
        }
    }

    fn set<T: 'static + Clone>(&self, key: &str, value: T) {
        let mut guard = self.data.lock().unwrap();
        guard.insert(key.to_string(), Box::new(Arc::new(value)));
    }

    fn get<T: 'static + Clone>(&self, key: &str) -> T {
        let guard = self.data.lock().unwrap();
        let value = guard.get(key).unwrap();
        value.downcast_ref::<T>().unwrap().clone()
    }
}
