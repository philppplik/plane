//! Fortschrittsdarstellung für Konsole und TUI.
//!
//! Der Balken besteht aus Blockzeichen statt aus einem Widget, damit CLI und
//! TUI dieselbe Berechnung verwenden und der Balken sich ohne Terminal testen
//! lässt.

use crate::engine::fsutil::format_bytes;
use crate::engine::Progress;
use crate::i18n;

use super::text;

/// Gefülltes Element des Balkens.
pub const VOLL: char = '█';
/// Leeres Element des Balkens.
pub const LEER: char = '░';

/// Prozentwert auf 0–100 begrenzen.
///
/// Die Engine rundet auf eine Nachkommastelle; NaN oder Werte jenseits der
/// Grenzen sollen die Ausgabe trotzdem nie zerstören.
pub fn begrenzen(prozent: f64) -> f64 {
    if prozent.is_nan() {
        0.0
    } else {
        prozent.clamp(0.0, 100.0)
    }
}

/// Balken aus Blockzeichen bauen.
///
/// Beispiel bei Breite 10 und 50 %: `█████░░░░░`.
pub fn balken(prozent: f64, breite: usize) -> String {
    if breite == 0 {
        return String::new();
    }
    let anteil = begrenzen(prozent) / 100.0;
    let gefuellt = (anteil * breite as f64).round() as usize;
    let gefuellt = gefuellt.min(breite);
    let mut text = String::with_capacity(breite * 3);
    for _ in 0..gefuellt {
        text.push(VOLL);
    }
    for _ in 0..(breite - gefuellt) {
        text.push(LEER);
    }
    text
}

/// Balken mit nachgestellter Prozentzahl, rechtsbündig auf drei Stellen.
pub fn balken_mit_prozent(prozent: f64, breite: usize) -> String {
    format!(
        "{} {:>3.0}%",
        balken(prozent, breite),
        begrenzen(prozent).floor()
    )
}

/// Vollständige Fortschrittszeile: Balken, Prozent, Zielname, Bytesumme.
///
/// `breite` ist die verfügbare Terminalbreite; die Zeile wird darauf gekürzt,
/// damit sie beim Überschreiben mit `\r` nicht umbricht und Reste stehen
/// lässt.
pub fn zeile(lang: &str, fortschritt: &Progress, breite: usize) -> String {
    let name = i18n::t(lang, &fortschritt.target_i18n);
    let phase = if fortschritt.phase == "clean" {
        i18n::t(lang, "home.cleaning")
    } else {
        i18n::t(lang, "cli.scanning")
    };
    let roh = format!(
        "{} {} {} [{}/{}] {}",
        balken_mit_prozent(fortschritt.percent, 20),
        phase,
        name,
        fortschritt.index + 1,
        fortschritt.total.max(1),
        format_bytes(fortschritt.bytes)
    );
    text::kuerzen(&roh, breite.max(20))
}

/// Zeile so ausgeben, dass die vorherige vollständig überschrieben wird.
///
/// Ohne die Leerzeichen bleiben Reste einer längeren vorherigen Zeile stehen –
/// ein klassischer Anzeigefehler bei `\r`-Fortschritt.
pub fn ueberschreibend(inhalt: &str, breite: usize) -> String {
    let laenge = inhalt.chars().count();
    let auffuellung = breite.saturating_sub(laenge);
    format!("\r{inhalt}{}", " ".repeat(auffuellung))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fortschritt(phase: &'static str, prozent: f64) -> Progress {
        Progress {
            phase,
            target_key: "system.temp.user".into(),
            target_i18n: "target.system.temp.user.name".into(),
            index: 0,
            total: 4,
            percent: prozent,
            bytes: 2048,
            current_path: String::new(),
            done: false,
        }
    }

    #[test]
    fn balken_bei_null_prozent_ist_leer() {
        assert_eq!(balken(0.0, 10), "░░░░░░░░░░");
    }

    #[test]
    fn balken_bei_fuenfzig_prozent_ist_halb_voll() {
        assert_eq!(balken(50.0, 10), "█████░░░░░");
    }

    #[test]
    fn balken_bei_hundert_prozent_ist_voll() {
        assert_eq!(balken(100.0, 10), "██████████");
    }

    #[test]
    fn balken_haelt_die_breite_immer_ein() {
        for prozent in [-50.0, 0.0, 33.3, 99.9, 100.0, 250.0] {
            assert_eq!(balken(prozent, 17).chars().count(), 17);
        }
    }

    #[test]
    fn balken_mit_breite_null_ist_leer() {
        assert_eq!(balken(50.0, 0), "");
    }

    #[test]
    fn prozent_wird_begrenzt() {
        assert_eq!(begrenzen(-1.0), 0.0);
        assert_eq!(begrenzen(101.0), 100.0);
        assert_eq!(begrenzen(f64::NAN), 0.0);
        assert_eq!(begrenzen(42.5), 42.5);
    }

    #[test]
    fn balken_mit_prozent_haengt_die_zahl_an() {
        assert!(balken_mit_prozent(50.0, 4).ends_with(" 50%"));
        assert!(balken_mit_prozent(0.0, 4).ends_with("  0%"));
        assert!(balken_mit_prozent(100.0, 4).ends_with("100%"));
    }

    #[test]
    fn fortschrittszeile_nennt_ziel_und_bytes() {
        let zeile = zeile("de", &fortschritt("scan", 25.0), 200);
        assert!(zeile.contains("Temporäre Dateien"));
        assert!(zeile.contains("[1/4]"));
        assert!(zeile.contains("KB") || zeile.contains("2"));
    }

    #[test]
    fn fortschrittszeile_unterscheidet_die_phasen() {
        let s = zeile("de", &fortschritt("scan", 10.0), 200);
        let c = zeile("de", &fortschritt("clean", 10.0), 200);
        assert_ne!(s, c);
    }

    #[test]
    fn fortschrittszeile_wird_auf_die_terminalbreite_gekuerzt() {
        let z = zeile("de", &fortschritt("scan", 10.0), 40);
        assert!(z.chars().count() <= 40);
    }

    #[test]
    fn ueberschreibende_zeile_fuellt_auf_die_breite_auf() {
        let ausgabe = ueberschreibend("abc", 6);
        assert_eq!(ausgabe, "\rabc   ");
        // Kürzer als der Inhalt darf nicht abschneiden.
        assert_eq!(ueberschreibend("abcdef", 3), "\rabcdef");
    }
}
