use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use specta::Type;
use specta_typescript::Typescript;
use tauri::{Emitter, Manager, State};
use tauri_specta::{collect_commands, Event};

mod state;

#[derive(Deserialize, Serialize, Type, Clone, Debug)]
pub struct InternalState {
    pub authenticated: bool,
    pub name: String,
}

type SharedInternalState = Mutex<InternalState>;

#[tauri::command]
#[specta::specta]
fn get_state(name: String, app: tauri::AppHandle) -> bool {
    println!("get_state: {:?}", name);

    match name.as_str() {
        "internal_state" => {
            let internal_state_ref = app.state::<SharedInternalState>();
            let guard = internal_state_ref.lock().unwrap();

            println!("emitting internal_state {:?}", guard.clone());
            app.emit("internal_state_update", guard.clone())
                .expect("unable to emit state");

            return true;
        }
        _ => return false,
    }
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
#[specta::specta]
fn greet(
    name: String,
    app: tauri::AppHandle,
    internal_state_ref: State<'_, SharedInternalState>,
) -> String {
    let mut internal_state = internal_state_ref.lock().unwrap();

    internal_state.authenticated = true;

    app.emit("internal_state_update", internal_state.clone())
        .expect("unable to emit state");

    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();

    let handlers = tauri_specta::Builder::<tauri::Wry>::new()
        .typ::<InternalState>()
        .commands(collect_commands![greet, get_state,])
        .events(tauri_specta::collect_events![state::Update]);

    #[cfg(debug_assertions)] // <- Only export on non-release builds
    handlers
        .export(
            Typescript::default()
                .formatter(specta_typescript::formatter::prettier)
                .bigint(specta_typescript::BigIntExportBehavior::BigInt)
                .header("/* eslint-disable */"),
            "../src/lib/bindings.ts",
        )
        .expect("Failed to export typescript bindings");

    let _builder = builder
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(handlers.invoke_handler())
        .setup(move |app| {
            // This is also required if you want to use events
            handlers.mount_events(app);

            let app_ref = app.handle().clone();

            state::Update::listen(app, move |event| {
                println!("state update handler: {:?}", event.payload);

                match event.payload.name.as_str() {
                    "internal_state" => {
                        let new_state: InternalState =
                            match serde_json::from_str(&event.payload.value) {
                                Ok(res) => res,
                                Err(_) => {
                                    println!("failed to parse internal state");
                                    return;
                                }
                            };

                        println!("internal state update: {:?}", new_state.clone());
                        let internal_state_ref = app_ref.state::<SharedInternalState>();
                        let mut guard = internal_state_ref.lock().unwrap();
                        *guard = new_state;
                    }
                    _ => return,
                }
            });

            let internal_state = InternalState {
                authenticated: false,
                name: "".to_owned(),
            };
            app.manage::<SharedInternalState>(Mutex::new(internal_state));

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
