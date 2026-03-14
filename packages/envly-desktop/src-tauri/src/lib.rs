mod commands;
mod core;
mod error;
mod state;

use std::fs;
use std::time::{Duration, Instant};

use tauri::{Manager, RunEvent};

use commands::environments::*;
use commands::projects::*;
use commands::secrets::*;
use commands::vault_commands::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data = app
                .path()
                .app_data_dir()
                .expect("Failed to resolve app data directory");
            fs::create_dir_all(&app_data).expect("Failed to create app data directory");

            let registry_path = app_data.join("registry.json");
            let mut registry = crate::core::registry::VaultRegistry::load(&registry_path)
                .expect("Failed to load vault registry");

            let pruned = registry.prune_stale();
            if !pruned.is_empty() {
                let _ = registry.save(&registry_path);
            }

            let tmp_dir = {
                #[cfg(target_os = "linux")]
                {
                    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
                        let dir = std::path::PathBuf::from(runtime_dir).join("com.envly.envly");
                        if fs::create_dir_all(&dir).is_ok() {
                            dir
                        } else {
                            app_data.join("tmp")
                        }
                    } else {
                        app_data.join("tmp")
                    }
                }
                #[cfg(not(target_os = "linux"))]
                {
                    app_data.join("tmp")
                }
            };
            fs::create_dir_all(&tmp_dir).expect("Failed to create tmp directory");

            let _ = crate::core::env::symlink::cleanup_stale(&tmp_dir);

            let app_state = state::AppState::new(registry_path, registry, tmp_dir);
            app.manage(app_state);

            // Background save loop: flush dirty vault every ~2s, force every 5 min
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let mut last_forced = Instant::now();
                loop {
                    std::thread::sleep(Duration::from_secs(2));
                    let state = handle.state::<state::AppState>();
                    if !state.is_unlocked() {
                        continue;
                    }
                    let force = last_forced.elapsed() >= Duration::from_secs(300);
                    if state.is_dirty() || force {
                        let _ = state.flush_if_dirty();
                        if force {
                            let _ = state.save();
                            last_forced = Instant::now();
                        }
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Registry / multi-vault
            list_vaults,
            get_active_vault_id,
            create_vault_entry,
            delete_vault_entry,
            rename_vault,
            import_vault_entry,
            select_vault,
            check_path_exists,
            // Vault lifecycle
            get_vault_status,
            unlock_vault,
            lock_vault,
            change_passphrase,
            export_vault,
            get_active_vault_cipher_kind,
            migrate_cipher,
            // Secrets
            list_secrets,
            reveal_secret_value,
            create_secret,
            update_secret,
            delete_secret,
            // Projects
            list_projects,
            create_project,
            update_project,
            delete_project,
            validate_project_path,
            toggle_project_starred,
            // Environments
            list_environments,
            list_mappings,
            create_environment,
            update_environment,
            delete_environment,
            activate_environment,
            deactivate_environment,
            add_mapping,
            remove_mapping,
            // Clone
            clone_environment,
            clone_project,
            // Bulk import
            parse_env_file,
            bulk_create_secrets,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| match event {
            RunEvent::ExitRequested { .. } | RunEvent::Exit => {
                let state = app_handle.state::<state::AppState>();
                if state.is_unlocked() {
                    let _ = state.lock();
                }
            }
            _ => {}
        });
}
