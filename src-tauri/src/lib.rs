//! Plane – PC-Cleaner für Windows.
//!
//! Die gesamte Anwendung liegt in dieser Bibliothek; `main.rs` ist nur ein
//! Einstiegspunkt. Dadurch wird der Code genau einmal kompiliert, die
//! Unit-Tests laufen einmal statt doppelt, und die Kommandozeile
//! (`src/bin/plane-cli.rs`) nutzt dieselbe Engine wie die Oberfläche.
//!
//! ```text
//! engine/    Analyse und Bereinigung – kennt weder Tauri noch eine UI
//! i18n       Sprachkatalog für GUI, CLI und TUI
//! state      Einstellungen und Zustand, als JSON persistiert
//! commands   Tauri-Brücke: sperren, delegieren, Fortschritt weiterreichen
//! ```

pub mod cli;
pub mod commands;
pub mod engine;
pub mod i18n;
pub mod state;

use std::sync::Mutex;

use tauri::Manager;

use state::AppState;

/// Anwendung aufbauen und starten.
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // Persistierten Zustand laden, bevor das Frontend Fragen stellt.
            let config_dir = app.path().app_config_dir().ok();
            let zustand = config_dir
                .as_deref()
                .map(AppState::load)
                .unwrap_or_default();

            app.manage(Mutex::new(zustand));
            app.manage(engine::CancelToken::new());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Navigation
            commands::start_app,
            commands::show_home,
            commands::show_about,
            commands::get_start_screen,
            // Einstellungen und Sprache
            commands::get_settings,
            commands::set_settings,
            commands::reset_welcome,
            commands::get_languages,
            commands::get_translations,
            // Informationen
            commands::get_brand,
            commands::list_targets,
            commands::get_disk_stats,
            commands::get_system_info,
            // Analyse und Bereinigung
            commands::scan,
            commands::clean,
            commands::cancel_run,
            // Rechte und Protokoll
            commands::restart_as_admin,
            commands::get_log,
            commands::get_log_path,
            // Programme
            commands::list_programs,
            commands::uninstall_program,
            commands::get_program_icons,
            // Windows-Einstellungen
            commands::list_tweaks,
            commands::set_tweak,
            commands::revert_tweak,
            // Aktualisierungsprüfung
            commands::check_update,
            commands::open_release_page,
            commands::skip_version,
        ])
        .run(tauri::generate_context!())
        .expect("Fehler beim Starten der Tauri-Anwendung");
}
