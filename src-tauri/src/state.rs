use std::fmt::Debug;
use std::sync::MutexGuard;
use std::{
    any::Any,
    collections::HashMap,
    pin::Pin,
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use specta::Type;
use tauri::{AppHandle, Emitter};
use tauri_specta::Event;

#[derive(Deserialize, Serialize, Type, Clone, Debug, Event)]
pub struct StateUpdate {
    pub version: Option<u128>,
    pub name: String,
    pub value: String,
}

pub struct Item<'r, T: Send + Sync + 'static>(&'r T);

impl<T: Send + Sync + 'static> Drop for Item<'_, T> {
    fn drop(&mut self) {
        debug!("item dropped")
    }
}

impl<T: Send + Sync + 'static> std::ops::Deref for Item<'_, T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &T {
        self.0
    }
}

impl<T: Send + Sync + 'static> Clone for Item<'_, T> {
    fn clone(&self) -> Self {
        Item(self.0)
    }
}

impl<T: Send + Sync + 'static + PartialEq> PartialEq for Item<'_, T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

macro_rules! update_handler {
    ($state:ty, $app_ref:expr, $payload_value:expr) => {{
        let new_state: $state = match serde_json::from_str($payload_value) {
            Ok(res) => res,
            Err(_) => {
                tracing::error!("failed to parse internal state");
                return;
            }
        };

        tracing::debug!("update {}: {:?}", stringify!($state), new_state.clone());
        let internal_state_ref = $app_ref.state::<std::sync::Mutex<$state>>();
        let mut guard = internal_state_ref.lock().unwrap();
        *guard = new_state;
    }};
}
use tracing::{debug, error, trace};
pub(crate) use update_handler;

macro_rules! emit_handler {
    ($state:ty, $app_ref:expr) => {{
        let state_ref = $app_ref.state::<std::sync::Mutex<$state>>();
        let guard = state_ref.lock().unwrap();

        let key = format!("{}_update", stringify!($state));
        tracing::debug!("emitting {}: {:?}", stringify!($state), guard.clone());
        $app_ref
            .emit(key.as_str(), guard.clone())
            .expect("unable to emit state");
        return true;
    }};
}
pub(crate) use emit_handler;

type MapAny = HashMap<String, Pin<Box<dyn Any + Send + Sync>>>;

#[derive(Clone)]
pub struct Syncer {
    data: Arc<Mutex<MapAny>>,
    app: AppHandle,
}

impl Syncer {
    pub fn new(app: AppHandle) -> Self {
        let syncer = Syncer {
            data: Default::default(),
            app: app.clone(),
        };

        syncer
    }

    pub fn update_string<'a, T>(&self, key: &str, value: &'a str)
    where
        T: 'static + Send + Sync + Serialize + Deserialize<'a> + Debug,
    {
        debug!(key, "update_string");
        let new_value: T = match serde_json::from_str(value) {
            Ok(res) => res,
            Err(_) => {
                error!("failed to parse internal state");
                return;
            }
        };

        self.update(key, new_value);
    }

    pub fn update<'a, T>(&self, key: &str, new_value: T)
    where
        T: 'static + Send + Sync + Serialize + Deserialize<'a> + Debug,
    {
        debug!(key, "update: {:?}", new_value);
        let mut guard = self.data.lock().unwrap();
        if !guard.contains_key(key) {
            debug!("key doesn't already exist, inserting instead");
            guard.insert(key.to_string(), Box::pin(Mutex::new(new_value)));
            return;
        }

        let ptr = guard.get(key).unwrap();
        let value = unsafe {
            ptr.downcast_ref::<Mutex<T>>()
                // SAFETY: the type of the key is the same as the type of the value
                .unwrap_unchecked()
        };
        let v_ref = unsafe { &*(value as *const Mutex<T>) };

        let mut v_guard = v_ref.lock().unwrap();
        *v_guard = new_value;
    }

    pub fn set<'a, T>(&self, key: &str, value: T)
    where
        T: 'static + Send + Sync + Serialize + Deserialize<'a>,
    {
        debug!(key, "set");
        let mut guard = self.data.lock().unwrap();
        guard.insert(key.to_string(), Box::pin(Mutex::new(value)));
    }

    pub fn get<'a, T>(&self, key: &str) -> Item<'_, Mutex<T>>
    where
        T: 'static + Send + Sync + Serialize + Deserialize<'a>,
    {
        debug!(key, "get");
        let guard = self.data.lock().unwrap();
        let ptr = guard.get(key).unwrap();
        let value = unsafe {
            ptr.downcast_ref::<Mutex<T>>()
                // SAFETY: the type of the key is the same as the type of the value
                .unwrap_unchecked()
        };
        let v_ref = unsafe { &*(value as *const Mutex<T>) };
        Item(v_ref)
    }

    pub fn emit<'a, T>(&self, name: &str) -> bool
    where
        T: 'static + Send + Sync + Serialize + Deserialize<'a> + Clone + Debug,
    {
        debug!(key = name, "emit");
        let guard = self.data.lock().unwrap();
        let ptr = guard.get(name).unwrap();
        let value = unsafe {
            ptr.downcast_ref::<Mutex<T>>()
                // SAFETY: the type of the key is the same as the type of the value
                .unwrap_unchecked()
        };
        let v_ref = unsafe { &*(value as *const Mutex<T>) };
        let value: MutexGuard<'_, T> = match v_ref.lock() {
            Ok(val) => val,
            Err(_) => return false,
        };

        let key = format!("{}_update", name);
        debug!("emitting {}: {:?}", name, value.clone());
        self.app
            .emit(key.as_str(), value.clone())
            .expect("unable to emit state");
        return true;
    }
}
