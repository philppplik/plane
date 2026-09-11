//! Systeminformationen für `plane-cli info`.
//!
//! Bewusst nicht aus `commands.rs` übernommen: die dortigen Funktionen sind
//! `#[tauri::command]` und asynchron, brauchen also eine laufende Tauri-App.
//! Die CLI soll ohne Fensterumgebung auskommen.

use serde::Serialize;
use sysinfo::{Disks, System};

use crate::engine::{self, fsutil};

/// Belegung eines Laufwerks.
#[derive(Debug, Clone, Serialize)]
pub struct Laufwerk {
    pub mount: String,
    pub total: u64,
    pub free: u64,
    pub used_percent: f64,
}

impl Laufwerk {
    /// Kennzahlen aus Gesamt- und Freigröße ableiten.
    ///
    /// Der Prozentwert wird berechnet statt gespeichert – sonst können die
    /// drei Zahlen auseinanderlaufen.
    pub fn aus_bytes(mount: String, total: u64, free: u64) -> Self {
        let used_percent = if total == 0 {
            0.0
        } else {
            ((total.saturating_sub(free)) as f64 / total as f64 * 1000.0).round() / 10.0
        };
        Self {
            mount,
            total,
            free,
            used_percent,
        }
    }
}

/// Zusammenstellung für die Info-Ausgabe.
#[derive(Debug, Clone, Serialize)]
pub struct Systembericht {
    pub os: String,
    pub arch: String,
    pub cpu: String,
    pub ram: u64,
    pub is_admin: bool,
    pub system_drive: String,
    pub target_count: usize,
    pub default_selection: usize,
    pub disks: Vec<Laufwerk>,
}

/// Systembericht erheben.
pub fn erheben() -> Systembericht {
    let mut system = System::new();
    system.refresh_memory();
    system.refresh_cpu_all();

    let cpu = system
        .cpus()
        .first()
        .map(|c| c.brand().trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| std::env::consts::ARCH.to_string());

    let mut disks: Vec<Laufwerk> = Disks::new_with_refreshed_list()
        .iter()
        .map(|d| {
            Laufwerk::aus_bytes(
                d.mount_point().to_string_lossy().to_string(),
                d.total_space(),
                d.available_space(),
            )
        })
        .collect();
    disks.sort_by(|a, b| a.mount.cmp(&b.mount));
    disks.dedup_by(|a, b| a.mount == b.mount);

    Systembericht {
        os: format!(
            "{} {}",
            System::name().unwrap_or_else(|| std::env::consts::OS.to_string()),
            System::os_version().unwrap_or_default()
        )
        .trim()
        .to_string(),
        arch: std::env::consts::ARCH.to_string(),
        cpu,
        ram: system.total_memory(),
        is_admin: engine::is_admin(),
        system_drive: fsutil::system_drive(),
        target_count: engine::TARGETS.len(),
        default_selection: engine::default_selection().len(),
        disks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn belegung_wird_aus_bytes_berechnet() {
        let l = Laufwerk::aus_bytes("C:\\".into(), 1000, 250);
        assert_eq!(l.used_percent, 75.0);
    }

    #[test]
    fn leeres_laufwerk_meldet_null_prozent_statt_nan() {
        let l = Laufwerk::aus_bytes("Z:\\".into(), 0, 0);
        assert_eq!(l.used_percent, 0.0);
        assert!(l.used_percent.is_finite());
    }

    #[test]
    fn systembericht_ist_vollstaendig() {
        let bericht = erheben();
        assert!(!bericht.arch.is_empty());
        assert!(!bericht.system_drive.is_empty());
        assert_eq!(bericht.target_count, engine::TARGETS.len());
        assert!(bericht.default_selection <= bericht.target_count);
    }
}
