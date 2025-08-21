use std::{
    any::Any,
    collections::HashMap,
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::AppHandle;
use tauri_specta::Event;

#[derive(Deserialize, Serialize, Type, Clone, Debug, Event)]
pub struct Update {
    pub version: Option<u128>,
    pub name: String,
    pub value: String,
}

macro_rules! update_handler {
    ($state:ty, $app_ref:expr, $payload_value:expr) => {{
        let new_state: $state = match serde_json::from_str($payload_value) {
            Ok(res) => res,
            Err(_) => {
                println!("failed to parse internal state");
                return;
            }
        };

        println!("update {}: {:?}", stringify!($state), new_state.clone());
        let internal_state_ref = $app_ref.state::<std::sync::Mutex<$state>>();
        let mut guard = internal_state_ref.lock().unwrap();
        *guard = new_state;
    }};
}
pub(crate) use update_handler;

macro_rules! emit_handler {
    ($state:ty, $app_ref:expr) => {{
        let state_ref = $app_ref.state::<std::sync::Mutex<$state>>();
        let guard = state_ref.lock().unwrap();

        let key = format!("{}_update", stringify!($state));
        println!("emitting {}: {:?}", stringify!($state), guard.clone());
        $app_ref
            .emit(key.as_str(), guard.clone())
            .expect("unable to emit state");
        return true;
    }};
}
pub(crate) use emit_handler;

pub struct StateSyncer {
    data: Mutex<HashMap<String, Box<Arc<dyn Any>>>>,
    app: AppHandle,
}

impl StateSyncer {
    fn new(app: AppHandle) -> Self {
        StateSyncer {
            data: Mutex::new(HashMap::new()),
            app,
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
