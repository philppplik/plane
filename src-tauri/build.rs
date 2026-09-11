//! Buildskript.
//!
//! Setzt zusaetzlich zum Tauri-Standardbuild die Umgebungsvariable
//! `PLANE_BUILD_DATE`, damit das Builddatum nicht im Quelltext gepflegt werden
//! muss (frueher: hart codiertes Literal in `commands.rs`).

use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    println!("cargo:rustc-env=PLANE_BUILD_DATE={}", build_date());
    tauri_build::build();
}

/// Aktuelles UTC-Datum als `YYYY-MM-DD`.
///
/// Bewusst ohne `chrono`: eine Datumsformatierung rechtfertigt keine weitere
/// Abhaengigkeit. Umrechnung nach dem Verfahren von Howard Hinnant
/// (`civil_from_days`).
fn build_date() -> String {
    let sekunden = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (jahr, monat, tag) = civil_from_days((sekunden / 86_400) as i64);
    format!("{jahr:04}-{monat:02}-{tag:02}")
}

fn civil_from_days(tage: i64) -> (i64, u32, u32) {
    let z = tage + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}
