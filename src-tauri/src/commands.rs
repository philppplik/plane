//! Tauri-Commands – Brücke zwischen Oberfläche und Engine.
//!
//! Die Commands enthalten keine Reinigungslogik. Sie
//!
//! * sperren den Zustand ohne `unwrap()` (ein vergifteter Mutex würde sonst
//!   jeden weiteren Aufruf abstürzen lassen),
//! * geben **typisierte** Werte zurück, nie handgebautes JSON,
//! * lagern rechenintensive Arbeit in `spawn_blocking` aus, damit die
//!   Oberfläche während Analyse und Bereinigung bedienbar bleibt,
//! * leiten Fortschritt als Event `plane://progress` an das Frontend weiter.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State, Window};

use crate::engine::{
    self,
    types::{CleanReport, CleanRequest, Progress, ScanReport},
};
use crate::i18n;
use crate::state::{AppState, Screen, Settings};

/// Eventname für Fortschrittsmeldungen.
pub const PROGRESS_EVENT: &str = "plane://progress";

/// Verwalteter Zustandstyp.
pub type SharedState = Mutex<AppState>;

/// Abbruchsignal des laufenden Vorgangs.
pub type SharedCancel = engine::CancelToken;

// ---------------------------------------------------------------------------
// Antworttypen
// ---------------------------------------------------------------------------

/// Markeninformationen.
#[derive(Serialize, Clone)]
pub struct BrandInfo {
    pub name: String,
    pub version: String,
    pub build_date: String,
    pub developer: String,
    pub colors: ColorPalette,
}

#[derive(Serialize, Clone)]
pub struct ColorPalette {
    pub bg: String,
    pub accent: String,
    pub text: String,
}

/// Laufwerkskennzahlen des Systemlaufwerks.
#[derive(Serialize, Clone, Debug)]
pub struct DiskStats {
    pub drive: String,
    pub total_gb: f64,
    pub free_gb: f64,
    pub used_percent: f64,
}

impl DiskStats {
    /// Kennzahlen aus Gesamt- und Freigröße in Bytes ableiten.
    ///
    /// `used_percent` wird **berechnet** und nicht separat gepflegt, damit die
    /// drei Werte nicht auseinanderlaufen können.
    pub fn from_bytes(drive: String, total: u64, free: u64) -> Self {
        let used_percent = if total == 0 {
            0.0
        } else {
            round1((total.saturating_sub(free)) as f64 / total as f64 * 100.0)
        };
        Self {
            drive,
            total_gb: round1(total as f64 / 1024_f64.powi(3)),
            free_gb: round1(free as f64 / 1024_f64.powi(3)),
            used_percent,
        }
    }
}

/// Systeminformationen für die Info-Ansicht.
#[derive(Serialize, Clone, Debug)]
pub struct SystemInfo {
    pub os: String,
    pub cpu: String,
    pub arch: String,
    pub ram_gb: f64,
    pub is_admin: bool,
    pub disk: DiskStats,
}

/// Beschreibung eines Reinigungsziels für die Oberfläche.
#[derive(Serialize, Clone, Debug)]
pub struct TargetInfo {
    pub key: String,
    pub category: engine::Category,
    pub name_key: String,
    pub description_key: String,
    pub risk: engine::Risk,
    pub requires_admin: bool,
    pub default_enabled: bool,
    pub suggestion_only: bool,
}

/// Sprachangebot für die Einstellungen.
#[derive(Serialize, Clone, Debug)]
pub struct LanguageInfo {
    pub code: String,
    pub label: String,
}

fn round1(wert: f64) -> f64 {
    (wert * 10.0).round() / 10.0
}

// ---------------------------------------------------------------------------
// Hilfsfunktionen
// ---------------------------------------------------------------------------

/// Zustand sperren, ohne bei Vergiftung zu panicken.
fn lock<'a>(state: &'a State<'_, SharedState>) -> Result<MutexGuard<'a, AppState>, String> {
    state
        .lock()
        .map_err(|_| "Interner Zustand ist beschädigt. Bitte Plane neu starten.".to_string())
}

fn config_dir(app: &AppHandle) -> PathBuf {
    app.path()
        .app_config_dir()
        .unwrap_or_else(|_| std::env::temp_dir().join("plane"))
}

/// Zustand persistieren; Fehler werden protokolliert, brechen die Bedienung
/// aber nicht ab – ein nicht schreibbares Profil darf die App nicht blockieren.
fn persist(app: &AppHandle, zustand: &AppState) {
    if let Err(e) = zustand.save(&config_dir(app)) {
        eprintln!("Zustand konnte nicht gespeichert werden: {e}");
    }
}

/// Laufkontext mit Fortschrittsweiterleitung an das Frontend.
fn run_context(app: &AppHandle, cancel: engine::CancelToken) -> engine::RunContext {
    let handle = app.clone();
    engine::RunContext::new()
        .with_cancel(cancel)
        .with_progress(Box::new(move |fortschritt: Progress| {
            // Ein fehlgeschlagenes Event darf den Lauf nicht abbrechen.
            let _ = handle.emit(PROGRESS_EVENT, fortschritt);
        }))
}

// ---------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------

fn wechsle(
    window: &Window,
    state: &State<'_, SharedState>,
    app: &AppHandle,
    ziel: Screen,
    erster_start_abschliessen: bool,
) -> Result<String, String> {
    let kopie = {
        let mut zustand = lock(state)?;
        zustand.current_screen = ziel;
        if erster_start_abschliessen {
            zustand.has_seen_welcome = true;
        }
        zustand.clone()
    };

    if erster_start_abschliessen {
        persist(app, &kopie);
    }

    window.emit(ziel.event(), ()).map_err(|e| e.to_string())?;
    Ok(ziel.as_str().to_string())
}

/// Vom Willkommensbildschirm zur Übersicht wechseln.
#[tauri::command]
pub fn start_app(
    window: Window,
    app: AppHandle,
    state: State<'_, SharedState>,
) -> Result<String, String> {
    wechsle(&window, &state, &app, Screen::Home, true)
}

/// Zur Übersicht wechseln.
#[tauri::command]
pub fn show_home(
    window: Window,
    app: AppHandle,
    state: State<'_, SharedState>,
) -> Result<String, String> {
    wechsle(&window, &state, &app, Screen::Home, false)
}

/// Zur Info-Ansicht wechseln.
#[tauri::command]
pub fn show_about(
    window: Window,
    app: AppHandle,
    state: State<'_, SharedState>,
) -> Result<String, String> {
    wechsle(&window, &state, &app, Screen::About, false)
}

/// Bildschirm, mit dem das Frontend starten soll.
#[tauri::command]
pub fn get_start_screen(state: State<'_, SharedState>) -> Result<String, String> {
    Ok(lock(&state)?.start_screen().as_str().to_string())
}

// ---------------------------------------------------------------------------
// Einstellungen und Sprache
// ---------------------------------------------------------------------------

/// Aktuelle Einstellungen.
#[tauri::command]
pub fn get_settings(state: State<'_, SharedState>) -> Result<Settings, String> {
    Ok(lock(&state)?.settings.clone())
}

/// Einstellungen setzen und sofort speichern.
#[tauri::command]
pub fn set_settings(
    app: AppHandle,
    state: State<'_, SharedState>,
    settings: Settings,
) -> Result<Settings, String> {
    let kopie = {
        let mut zustand = lock(&state)?;
        zustand.settings = Settings {
            language: i18n::normalize(&settings.language),
            ..settings
        };
        zustand.clone()
    };
    persist(&app, &kopie);
    Ok(kopie.settings)
}

/// Willkommensbildschirm beim nächsten Start wieder zeigen.
#[tauri::command]
pub fn reset_welcome(app: AppHandle, state: State<'_, SharedState>) -> Result<(), String> {
    let kopie = {
        let mut zustand = lock(&state)?;
        zustand.has_seen_welcome = false;
        zustand.clone()
    };
    persist(&app, &kopie);
    Ok(())
}

/// Verfügbare Sprachen.
#[tauri::command]
pub fn get_languages() -> Vec<LanguageInfo> {
    i18n::LANGUAGES
        .iter()
        .map(|(code, label)| LanguageInfo {
            code: (*code).to_string(),
            label: (*label).to_string(),
        })
        .collect()
}

/// Alle Texte einer Sprache.
#[tauri::command]
pub fn get_translations(language: String) -> BTreeMap<String, String> {
    i18n::catalog(&i18n::normalize(&language))
}

// ---------------------------------------------------------------------------
// Informationen
// ---------------------------------------------------------------------------

/// Markeninformationen.
#[tauri::command]
pub fn get_brand() -> BrandInfo {
    BrandInfo {
        name: "Plane".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        // Wird vom Buildskript gesetzt, nicht im Quelltext gepflegt.
        build_date: env!("PLANE_BUILD_DATE").to_string(),
        developer: "Philipp Paulik".to_string(),
        colors: ColorPalette {
            bg: "#F1FFE2".to_string(),
            accent: "#A7EC5B".to_string(),
            text: "#143302".to_string(),
        },
    }
}

/// Alle Reinigungsziele des Katalogs.
#[tauri::command]
pub fn list_targets() -> Vec<TargetInfo> {
    engine::TARGETS
        .iter()
        .map(|t| TargetInfo {
            key: t.key.to_string(),
            category: t.category,
            name_key: t.i18n_name(),
            description_key: t.i18n_description(),
            risk: t.risk,
            requires_admin: t.requires_admin,
            default_enabled: t.default_enabled,
            suggestion_only: t.is_suggestion_only(),
        })
        .collect()
}

fn disk_stats_intern() -> DiskStats {
    use sysinfo::Disks;

    let laufwerk = engine::fsutil::system_drive();
    let disks = Disks::new_with_refreshed_list();
    let treffer = disks.iter().find(|d| {
        d.mount_point()
            .to_string_lossy()
            .eq_ignore_ascii_case(&laufwerk)
    });

    match treffer {
        Some(d) => DiskStats::from_bytes(laufwerk, d.total_space(), d.available_space()),
        None => DiskStats::from_bytes(laufwerk, 0, 0),
    }
}

/// Belegung des Systemlaufwerks.
#[tauri::command]
pub async fn get_disk_stats() -> Result<DiskStats, String> {
    tauri::async_runtime::spawn_blocking(disk_stats_intern)
        .await
        .map_err(|e| e.to_string())
}

/// Systeminformationen.
#[tauri::command]
pub async fn get_system_info() -> Result<SystemInfo, String> {
    tauri::async_runtime::spawn_blocking(|| {
        use sysinfo::System;

        let mut system = System::new();
        system.refresh_memory();
        system.refresh_cpu_all();

        let cpu = system
            .cpus()
            .first()
            .map(|c| c.brand().trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| std::env::consts::ARCH.to_string());

        SystemInfo {
            os: os_bezeichnung(),
            cpu,
            arch: std::env::consts::ARCH.to_string(),
            ram_gb: round1(system.total_memory() as f64 / 1024_f64.powi(3)),
            is_admin: engine::is_admin(),
            disk: disk_stats_intern(),
        }
    })
    .await
    .map_err(|e| e.to_string())
}

/// Betriebssystembezeichnung inklusive Build.
///
/// Windows 11 meldet weiterhin die Hauptversion 10; unterschieden wird daher
/// über die Buildnummer (>= 22000 = Windows 11).
fn os_bezeichnung() -> String {
    let Some(version) = sysinfo::System::os_version() else {
        return sysinfo::System::name().unwrap_or_else(|| "Unbekannt".to_string());
    };

    if !cfg!(windows) {
        return format!(
            "{} {version}",
            sysinfo::System::name().unwrap_or_else(|| "System".to_string())
        );
    }

    let build: u32 = sysinfo::System::kernel_version()
        .and_then(|k| k.split('.').next_back().and_then(|b| b.parse().ok()))
        .unwrap_or(0);
    let haupt = if build >= 22_000 {
        "11".to_string()
    } else {
        version.split('.').next().unwrap_or("10").to_string()
    };
    format!("Windows {haupt} (Build {build})")
}

// ---------------------------------------------------------------------------
// Analyse und Bereinigung
// ---------------------------------------------------------------------------

/// Ziele analysieren, ohne etwas zu verändern.
///
/// Leere Zielliste = gesamter Katalog.
#[tauri::command]
pub async fn scan(
    app: AppHandle,
    cancel: State<'_, SharedCancel>,
    targets: Vec<String>,
) -> Result<ScanReport, String> {
    let token = cancel.inner().clone();
    token.reset();
    let ctx = run_context(&app, token);

    let verzeichnis = config_dir(&app);
    let bericht = tauri::async_runtime::spawn_blocking(move || engine::scan(&targets, &ctx))
        .await
        .map_err(|e| e.to_string())?;

    engine::log::scan(&verzeichnis, &bericht);
    Ok(bericht)
}

/// Ausgewählte Ziele bereinigen.
#[tauri::command]
pub async fn clean(
    app: AppHandle,
    cancel: State<'_, SharedCancel>,
    request: CleanRequest,
) -> Result<CleanReport, String> {
    let token = cancel.inner().clone();
    token.reset();
    let ctx = run_context(&app, token);
    let verzeichnis = config_dir(&app);
    let backup_dir = crate::state::backup_path(&verzeichnis);
    let trockenlauf = request.dry_run;

    let bericht =
        tauri::async_runtime::spawn_blocking(move || engine::clean(&request, &backup_dir, &ctx))
            .await
            .map_err(|e| e.to_string())?;

    engine::log::clean(&verzeichnis, &bericht, trockenlauf);
    Ok(bericht)
}

/// Laufenden Vorgang abbrechen.
#[tauri::command]
pub fn cancel_run(cancel: State<'_, SharedCancel>) {
    cancel.cancel();
}

// ---------------------------------------------------------------------------
// Rechte und Protokoll
// ---------------------------------------------------------------------------

/// Plane mit Administratorrechten neu starten.
///
/// Der aufrufende Prozess beendet sich, sobald der neue gestartet ist – sonst
/// liefen zwei Instanzen nebeneinander. Lehnt der Nutzer die UAC-Rückfrage ab,
/// passiert nichts und der Fehler kommt als Übersetzungsschlüssel zurück.
#[tauri::command]
pub fn restart_as_admin(app: AppHandle) -> Result<String, String> {
    match engine::neu_starten_als_admin(&[]) {
        engine::Elevation::Bereits => Ok("already".to_string()),
        engine::Elevation::Gestartet => {
            engine::log::notiz(&config_dir(&app), "NEUSTART als Administrator");
            let handle = app.clone();
            // Kurz warten, damit die Antwort das Frontend noch erreicht.
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(300));
                handle.exit(0);
            });
            Ok("restarting".to_string())
        }
        engine::Elevation::Abgelehnt(grund) => {
            eprintln!("Neustart als Administrator: {grund}");
            Err("admin.failed".to_string())
        }
    }
}

/// Inhalt des Protokolls, neueste Zeilen zuletzt.
#[tauri::command]
pub fn get_log(app: AppHandle, lines: Option<usize>) -> Result<String, String> {
    let pfad = engine::log::log_path(&config_dir(&app));
    let Ok(inhalt) = std::fs::read_to_string(&pfad) else {
        return Ok(String::new());
    };

    let grenze = lines.unwrap_or(200);
    let zeilen: Vec<&str> = inhalt.lines().collect();
    let anfang = zeilen.len().saturating_sub(grenze);
    Ok(zeilen[anfang..].join("\n"))
}

// ---------------------------------------------------------------------------
// Programme deinstallieren
// ---------------------------------------------------------------------------

/// Installierte Programme auflisten.
///
/// Läuft über `spawn_blocking`: das Auslesen der Registry und der Store-Pakete
/// dauert je nach System ein bis zwei Sekunden.
#[tauri::command]
pub async fn list_programs() -> Result<Vec<engine::uninstall::Programm>, String> {
    tauri::async_runtime::spawn_blocking(engine::uninstall::liste)
        .await
        .map_err(|e| e.to_string())
}

/// Ein Programm deinstallieren.
///
/// `quiet` versucht eine Deinstallation ohne Dialog – das geht nur, wenn der
/// Hersteller einen stillen Schalter hinterlegt hat oder es ein MSI-Paket ist.
/// Ansonsten öffnet der Deinstaller sein eigenes Fenster.
#[tauri::command]
pub async fn uninstall_program(
    app: AppHandle,
    program: engine::uninstall::Programm,
    quiet: bool,
) -> Result<engine::uninstall::UninstallResult, String> {
    let verzeichnis = config_dir(&app);
    let name = program.name.clone();

    let ergebnis = tauri::async_runtime::spawn_blocking(move || {
        engine::uninstall::deinstalliere(&program, quiet)
    })
    .await
    .map_err(|e| e.to_string())?;

    engine::log::notiz(
        &verzeichnis,
        &format!(
            "DEINSTALLATION {name} ok={} exit={} geprueft={}",
            ergebnis.ok, ergebnis.exit_code, ergebnis.verified
        ),
    );

    Ok(ergebnis)
}

/// Pfad der Protokolldatei – damit die Oberfläche ihn anzeigen kann.
#[tauri::command]
pub fn get_log_path(app: AppHandle) -> String {
    engine::log::log_path(&config_dir(&app))
        .to_string_lossy()
        .to_string()
}

#[cfg(test)]
mod tests {
    //! Unit-Tests der Command-Schicht.
    //!
    //! Commands mit `Window`/`State` benötigen eine laufende Tauri-Instanz und
    //! sind hier nicht abgedeckt; getestet werden die Datenstrukturen und die
    //! zustandslosen Commands.

    use super::*;

    #[test]
    fn brand_info_serialisiert_alle_felder_die_das_frontend_erwartet() {
        let json = serde_json::to_value(get_brand()).unwrap();
        for feld in ["name", "version", "build_date", "developer", "colors"] {
            assert!(json.get(feld).is_some(), "Feld fehlt im JSON: {feld}");
        }
        for feld in ["bg", "accent", "text"] {
            assert!(json["colors"].get(feld).is_some(), "Farbe fehlt: {feld}");
        }
    }

    #[test]
    fn get_brand_liefert_version_aus_cargo_toml() {
        assert_eq!(get_brand().version, env!("CARGO_PKG_VERSION"));
        assert_eq!(get_brand().name, "Plane");
    }

    #[test]
    fn build_datum_kommt_aus_dem_buildskript() {
        let datum = get_brand().build_date;
        assert_eq!(datum.len(), 10, "unerwartetes Format: {datum}");
        let teile: Vec<&str> = datum.split('-').collect();
        assert_eq!(teile.len(), 3);
        assert!(teile[0].parse::<u32>().unwrap() >= 2024);
    }

    #[test]
    fn get_brand_liefert_gueltige_hex_farben() {
        let brand = get_brand();
        for farbe in [&brand.colors.bg, &brand.colors.accent, &brand.colors.text] {
            assert!(farbe.starts_with('#'), "kein Hex-Wert: {farbe}");
            assert_eq!(farbe.len(), 7);
            assert!(farbe[1..].chars().all(|c| c.is_ascii_hexdigit()));
        }
    }

    #[test]
    fn used_percent_wird_aus_total_und_free_berechnet() {
        let gb = 1024_u64.pow(3);
        let stats = DiskStats::from_bytes("C:\\".into(), 100 * gb, 40 * gb);
        assert_eq!(stats.total_gb, 100.0);
        assert_eq!(stats.free_gb, 40.0);
        assert_eq!(stats.used_percent, 60.0);
    }

    #[test]
    fn leeres_laufwerk_fuehrt_nicht_zur_division_durch_null() {
        assert_eq!(DiskStats::from_bytes("C:\\".into(), 0, 0).used_percent, 0.0);
    }

    #[test]
    fn disk_stats_serialisieren_nach_snake_case() {
        let json = serde_json::to_value(DiskStats::from_bytes("C:\\".into(), 0, 0)).unwrap();
        for feld in ["drive", "total_gb", "free_gb", "used_percent"] {
            assert!(json.get(feld).is_some(), "Feld fehlt: {feld}");
        }
    }

    #[test]
    fn list_targets_bildet_den_katalog_vollstaendig_ab() {
        let ziele = list_targets();
        assert_eq!(ziele.len(), engine::TARGETS.len());

        let papierkorb = ziele.iter().find(|t| t.key == "recyclebin.all").unwrap();
        assert_eq!(papierkorb.risk, engine::Risk::Caution);
        assert!(!papierkorb.default_enabled);
        assert_eq!(papierkorb.name_key, "target.recyclebin.all.name");
    }

    #[test]
    fn list_targets_liefert_aufloesbare_uebersetzungsschluessel() {
        for ziel in list_targets() {
            for key in [&ziel.name_key, &ziel.description_key] {
                assert_ne!(i18n::t("de", key), *key, "unübersetzt: {key}");
                assert_ne!(i18n::t("en", key), *key, "unübersetzt: {key}");
            }
        }
    }

    #[test]
    fn sprachliste_enthaelt_deutsch_und_englisch() {
        let sprachen = get_languages();
        assert_eq!(sprachen.len(), 2);
        assert!(sprachen
            .iter()
            .any(|s| s.code == "de" && s.label == "Deutsch"));
        assert!(sprachen.iter().any(|s| s.code == "en"));
    }

    #[test]
    fn uebersetzungen_kommen_in_der_angeforderten_sprache() {
        let de = get_translations("de-DE".into());
        let en = get_translations("en".into());
        assert_eq!(de.len(), en.len());
        assert_ne!(de["nav.settings"], en["nav.settings"]);
        assert_eq!(de["nav.settings"], "Einstellungen");
    }

    #[test]
    fn unbekannte_sprache_liefert_englisch() {
        assert_eq!(
            get_translations("fr".into())["nav.settings"],
            get_translations("en".into())["nav.settings"]
        );
    }

    #[test]
    fn scanreport_serialisiert_das_erwartete_schema() {
        let bericht = engine::scan(
            &["system.dns".to_string()],
            &engine::RunContext::new().with_elevated(false),
        );
        let json = serde_json::to_value(&bericht).unwrap();
        for feld in [
            "targets",
            "total_size",
            "total_items",
            "duration_ms",
            "cancelled",
        ] {
            assert!(json.get(feld).is_some(), "Feld fehlt: {feld}");
        }
    }

    #[test]
    fn cleanrequest_laesst_sich_aus_dem_frontend_json_lesen() {
        let anfrage: CleanRequest = serde_json::from_str(
            r#"{"targets":["system.temp.user"],"only_paths":[],"dry_run":true}"#,
        )
        .unwrap();
        assert_eq!(anfrage.targets, vec!["system.temp.user"]);
        assert!(anfrage.dry_run);
    }

    #[test]
    fn cleanrequest_hat_sinnvolle_standardwerte() {
        let anfrage: CleanRequest = serde_json::from_str(r#"{"targets":[]}"#).unwrap();
        assert!(anfrage.only_paths.is_empty());
        assert!(!anfrage.dry_run, "Trockenlauf darf nicht der Default sein");
    }

    #[tokio::test]
    async fn get_disk_stats_liefert_plausible_werte() {
        let stats = get_disk_stats().await.unwrap();
        assert!(stats.total_gb >= 0.0);
        assert!((0.0..=100.0).contains(&stats.used_percent));
        assert!(!stats.drive.is_empty());
    }

    #[tokio::test]
    async fn get_system_info_liefert_alle_felder() {
        let info = get_system_info().await.unwrap();
        assert!(!info.os.is_empty());
        assert!(!info.arch.is_empty());
        assert!(info.ram_gb > 0.0);
    }

    #[test]
    fn progress_event_heisst_wie_im_frontend() {
        assert_eq!(PROGRESS_EVENT, "plane://progress");
    }
}
