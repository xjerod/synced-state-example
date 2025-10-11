use serde::{Deserialize, Serialize};
use specta::Type;
use specta_typescript::Typescript;
use tauri::{Manager, State};
use tauri_specta::{collect_commands, Event};
use tauri_svelte_synced_store::{StateSyncer, StateUpdate};
use tracing::{info, warn};

#[derive(Clone, Deserialize, Serialize, Type, Debug)]
pub struct InternalState {
    pub authenticated: bool,
    pub name: String,
}

tauri_svelte_synced_store::state_handlers!(InternalState = "InternalState");

#[tauri::command]
#[specta::specta]
fn greet(name: String, _app: tauri::AppHandle, state_syncer: State<'_, StateSyncer>) -> String {
    info!(name, "greet");

    let internal_state_ref = state_syncer.get::<InternalState>("InternalState");
    let mut internal_state = internal_state_ref.lock().unwrap();

    internal_state.authenticated = true;

    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    color_eyre::install().expect("failed to install color_eyre");

    tracing_subscriber::fmt()
        // enable everything
        .with_max_level(tracing::Level::DEBUG)
        // sets this to be the default, global collector for this application.
        .init();

    let builder = tauri::Builder::default();

    let handlers = tauri_specta::Builder::<tauri::Wry>::new()
        .typ::<InternalState>()
        .commands(collect_commands![greet, emit_state, update_state,])
        .events(tauri_specta::collect_events![StateUpdate]);

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
            handlers.mount_events(app);

            let app_ref = app.handle().clone();

            let state_syncer = StateSyncer::new(app_ref.clone());
            let state_syncer_ref = state_syncer.clone();

            tauri_svelte_synced_store::state_listener!(
                app_ref.clone(),
                state_syncer_ref.clone(),
                InternalState = "InternalState"
            );

            state_syncer.set(
                "InternalState",
                InternalState {
                    authenticated: false,
                    name: "".to_owned(),
                },
            );
            app.manage::<StateSyncer>(state_syncer);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
