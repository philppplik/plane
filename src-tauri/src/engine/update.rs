//! Nach einer neueren Version sehen.
//!
//! # Die einzige Stelle mit Netzwerkzugriff
//!
//! Plane hatte bis Version 0.2.0 **keinen** Netzwerkcode. Diese Datei ist die
//! Ausnahme, und sie bleibt die einzige. Damit das Versprechen trotzdem hält,
//! gelten drei Regeln:
//!
//! 1. **Standardmäßig aus.** `Settings::check_updates` ist `false`. Wer nichts
//!    einstellt, für den verhält sich Plane wie vorher: kein Paket verlässt
//!    das Gerät.
//! 2. **Nur lesen, nur GitHub.** Ein einziger `GET` auf
//!    `api.github.com/repos/.../releases/latest`. Plane lädt nichts herunter,
//!    installiert nichts und führt nichts aus. Wer aktualisieren will,
//!    bekommt die Veröffentlichungsseite im Browser geöffnet.
//! 3. **Nichts wird mitgeschickt.** Keine Kennung, keine Systemdaten, kein
//!    Zählwerk. Die Anfrage enthält den `User-Agent` `plane/<version>` — das
//!    verlangt die GitHub-API — und sonst nichts. GitHub sieht die
//!    IP-Adresse, wie bei jedem Aufruf einer Webseite auch.
//!
//! # Warum kein eingebauter Installer
//!
//! Ein Programm, das sich selbst ersetzen kann, ist ein Programm, das sich
//! auch durch etwas anderes ersetzen lässt. Solange die Pakete nicht signiert
//! sind (siehe `SECURITY.md`), wäre eine automatische Installation ein
//! Angriffsweg und kein Komfortgewinn. Plane sagt deshalb nur Bescheid.

use serde::{Deserialize, Serialize};

/// Projektadresse – auch für die Anzeige in der Oberfläche.
pub const REPO: &str = "philppplik/plane";

/// Adresse der jüngsten Veröffentlichung.
///
/// `releases/latest` überspringt Entwürfe und Vorabversionen. Wer einen
/// Entwurf prüfen will, findet ihn ohnehin nur angemeldet.
const API: &str = "https://api.github.com/repos/philppplik/plane/releases/latest";

/// Seite, die beim Klick auf „Herunterladen" geöffnet wird.
const SEITE: &str = "https://github.com/philppplik/plane/releases/latest";

/// Wie lange auf GitHub gewartet wird, bevor abgebrochen wird.
///
/// Lieber eine ehrliche Fehlermeldung als eine Oberfläche, die eine halbe
/// Minute lang steht.
const ZEITGRENZE: std::time::Duration = std::time::Duration::from_secs(8);

/// Ergebnis einer Prüfung.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateInfo {
    /// Version dieses Programms.
    pub current: String,
    /// Jüngste veröffentlichte Version, ohne führendes `v`.
    pub latest: String,
    /// Ist `latest` tatsächlich neuer als `current`?
    pub newer: bool,
    /// Seite, auf der die Dateien liegen.
    pub url: String,
    /// Veröffentlichungsdatum als `YYYY-MM-DD`, falls vorhanden.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub published: String,
}

// ---------------------------------------------------------------------------
// Versionsvergleich
// ---------------------------------------------------------------------------

/// Drei Zahlen nach Semantic Versioning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(u32, u32, u32);

impl Version {
    /// `v1.2.3`, `1.2.3`, `1.2` und `1.2.3-beta.1` werden verstanden.
    ///
    /// Der Vorabteil hinter `-` wird **abgeschnitten**, nicht ausgewertet:
    /// Plane veröffentlicht keine Vorabversionen, und eine halbherzige
    /// Auswertung wäre schlechter als eine bewusste Auslassung.
    pub fn parse(text: &str) -> Option<Self> {
        let kern = text
            .trim()
            .trim_start_matches(['v', 'V'])
            .split(['-', '+'])
            .next()?;

        let mut zahlen = [0u32; 3];
        let mut gesehen = 0;

        for (stelle, teil) in kern.split('.').enumerate() {
            if stelle >= 3 {
                break;
            }
            zahlen[stelle] = teil.parse().ok()?;
            gesehen += 1;
        }

        if gesehen == 0 {
            return None;
        }
        Some(Version(zahlen[0], zahlen[1], zahlen[2]))
    }

    pub fn anzeige(self) -> String {
        format!("{}.{}.{}", self.0, self.1, self.2)
    }
}

/// Version dieses Programms, wie sie in `Cargo.toml` steht.
pub fn eigene_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

// ---------------------------------------------------------------------------
// Abfrage
// ---------------------------------------------------------------------------

/// Bei GitHub nach der jüngsten Veröffentlichung fragen.
///
/// Blockiert bis zu [`ZEITGRENZE`]. Aufrufer sollen das in einen eigenen
/// Thread legen — in der Oberfläche über `spawn_blocking`.
///
/// Fehler sind Übersetzungsschlüssel, keine fertigen Sätze: die Meldung
/// erscheint in der Sprache, die der Nutzer eingestellt hat.
pub fn pruefe() -> Result<UpdateInfo, String> {
    let antwort = hole(API)?;
    deute(&antwort)
}

/// HTTP-Teil, getrennt von der Auswertung — so lässt sich Letztere testen,
/// ohne das Netz zu berühren.
fn hole(adresse: &str) -> Result<String, String> {
    // Der Anbieter muss ausdrücklich gesetzt werden: ureq nimmt sonst Rustls
    // an und bricht zur Laufzeit ab, weil Plane dieses Merkmal nicht
    // einschaltet. Verschlüsselt wird über SChannel, also über den TLS-Stack
    // und den Zertifikatspeicher von Windows.
    let tls = ureq::tls::TlsConfig::builder()
        .provider(ureq::tls::TlsProvider::NativeTls)
        .build();

    let agent = ureq::Agent::config_builder()
        .tls_config(tls)
        .timeout_global(Some(ZEITGRENZE))
        // Umleitungen sind hier nicht nötig; GitHub antwortet direkt.
        .max_redirects(2)
        .build()
        .new_agent();

    let mut antwort = agent
        .get(adresse)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", &format!("plane/{}", eigene_version()))
        .call()
        .map_err(|fehler| match fehler {
            ureq::Error::StatusCode(404) => "update.error.none_published".to_string(),
            ureq::Error::StatusCode(403) | ureq::Error::StatusCode(429) => {
                "update.error.rate_limit".to_string()
            }
            ureq::Error::Timeout(_) => "update.error.timeout".to_string(),
            _ => "update.error.offline".to_string(),
        })?;

    antwort
        .body_mut()
        .read_to_string()
        .map_err(|_| "update.error.unreadable".to_string())
}

/// Antwort von GitHub auswerten.
fn deute(rohtext: &str) -> Result<UpdateInfo, String> {
    let daten: serde_json::Value =
        serde_json::from_str(rohtext).map_err(|_| "update.error.unreadable".to_string())?;

    let tag = daten
        .get("tag_name")
        .and_then(|w| w.as_str())
        .ok_or_else(|| "update.error.none_published".to_string())?;

    let neueste = Version::parse(tag).ok_or_else(|| "update.error.unreadable".to_string())?;
    let eigene = Version::parse(eigene_version()).unwrap_or(Version(0, 0, 0));

    // Die Seite aus der Antwort wird **nicht** übernommen: eine Adresse aus
    // einer Netzwerkantwort in den Browser zu reichen wäre ein unnötiger
    // Vertrauensschritt. Die Zieladresse steht fest im Programm.
    Ok(UpdateInfo {
        current: eigene.anzeige(),
        latest: neueste.anzeige(),
        newer: neueste > eigene,
        url: SEITE.to_string(),
        published: daten
            .get("published_at")
            .and_then(|w| w.as_str())
            .and_then(|w| w.split('T').next())
            .unwrap_or_default()
            .to_string(),
    })
}

// ---------------------------------------------------------------------------
// Seite öffnen
// ---------------------------------------------------------------------------

/// Die Veröffentlichungsseite im Standardbrowser öffnen.
///
/// Es gibt bewusst **keinen** Parameter: die Adresse steht fest im Programm.
/// Eine Funktion, die eine beliebige Adresse öffnet, wäre über die
/// Tauri-Brücke aus dem Frontend erreichbar — und damit ein Hebel, falls dort
/// je fremder Inhalt landet.
#[cfg(windows)]
pub fn oeffne_seite() -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;

    fn weit(text: &str) -> Vec<u16> {
        std::ffi::OsStr::new(text)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    let verb = weit("open");
    let adresse = weit(SEITE);

    // SAFETY: Beide Zeiger verweisen auf nullterminierte Puffer, die bis zum
    // Ende des Aufrufs am Leben bleiben. ShellExecuteW schreibt nicht hinein.
    let ergebnis = unsafe {
        windows_sys::Win32::UI::Shell::ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            adresse.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
        )
    };

    if ergebnis as isize > 32 {
        Ok(())
    } else {
        Err("update.error.browser".to_string())
    }
}

#[cfg(not(windows))]
pub fn oeffne_seite() -> Result<(), String> {
    Err("update.error.browser".to_string())
}

/// Adresse der Veröffentlichungsseite – für die Anzeige zum Abtippen.
pub fn seite() -> &'static str {
    SEITE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versionen_werden_in_allen_ueblichen_schreibweisen_gelesen() {
        assert_eq!(Version::parse("v1.2.3"), Some(Version(1, 2, 3)));
        assert_eq!(Version::parse("1.2.3"), Some(Version(1, 2, 3)));
        assert_eq!(Version::parse("V0.2.0"), Some(Version(0, 2, 0)));
        assert_eq!(Version::parse(" 1.2.3 "), Some(Version(1, 2, 3)));
        assert_eq!(Version::parse("1.2"), Some(Version(1, 2, 0)));
        assert_eq!(Version::parse("2"), Some(Version(2, 0, 0)));
    }

    #[test]
    fn vorabkennung_wird_abgeschnitten() {
        assert_eq!(Version::parse("v1.2.3-beta.1"), Some(Version(1, 2, 3)));
        assert_eq!(Version::parse("1.2.3+build7"), Some(Version(1, 2, 3)));
    }

    #[test]
    fn unsinn_ergibt_keine_version() {
        assert_eq!(Version::parse(""), None);
        assert_eq!(Version::parse("nightly"), None);
        assert_eq!(Version::parse("v.x.y"), None);
    }

    /// Der Kernfehler jedes selbstgebauten Vergleichs: `0.10.0` ist neuer als
    /// `0.9.0`, als Zeichenkette verglichen aber kleiner.
    #[test]
    fn zehn_ist_groesser_als_neun() {
        let neu = Version::parse("0.10.0").unwrap();
        let alt = Version::parse("0.9.0").unwrap();
        assert!(neu > alt);
        assert!("0.10.0" < "0.9.0", "Annahme des Tests stimmt nicht mehr");
    }

    #[test]
    fn gleiche_version_ist_nicht_neuer() {
        let a = Version::parse("1.0.0").unwrap();
        assert!(!(a > a));
    }

    #[test]
    fn neuere_veroeffentlichung_wird_erkannt() {
        let roh = r#"{"tag_name":"v99.0.0","published_at":"2030-01-02T03:04:05Z"}"#;
        let info = deute(roh).unwrap();
        assert_eq!(info.latest, "99.0.0");
        assert!(info.newer);
        assert_eq!(info.published, "2030-01-02");
        assert_eq!(
            info.current,
            Version::parse(eigene_version()).unwrap().anzeige()
        );
    }

    #[test]
    fn aeltere_veroeffentlichung_loest_keine_meldung_aus() {
        let info = deute(r#"{"tag_name":"v0.0.1"}"#).unwrap();
        assert!(!info.newer);
        assert_eq!(info.published, "");
    }

    #[test]
    fn eigene_version_meldet_sich_nicht_selbst_als_neuer() {
        let roh = format!(r#"{{"tag_name":"v{}"}}"#, eigene_version());
        assert!(!deute(&roh).unwrap().newer);
    }

    /// Die Adresse kommt aus dem Programm, nicht aus der Antwort. Sonst
    /// könnte eine manipulierte Antwort den Browser irgendwohin schicken.
    #[test]
    fn adresse_stammt_nicht_aus_der_antwort() {
        let roh = r#"{"tag_name":"v9.9.9","html_url":"https://example.invalid/boes"}"#;
        let info = deute(roh).unwrap();
        assert_eq!(info.url, SEITE);
        assert!(info.url.starts_with("https://github.com/"));
    }

    #[test]
    fn fehlerhafte_antworten_ergeben_uebersetzbare_schluessel() {
        assert_eq!(deute("kein json"), Err("update.error.unreadable".into()));
        assert_eq!(deute("{}"), Err("update.error.none_published".into()));
        assert_eq!(
            deute(r#"{"tag_name":"nightly"}"#),
            Err("update.error.unreadable".into())
        );
    }

    /// Regressionstest für einen Absturz, der nur zur Laufzeit auftrat.
    ///
    /// ureq nimmt ohne ausdrückliche Angabe Rustls als TLS-Anbieter an. Ist
    /// nur `native-tls` eingeschaltet, **panickt** es beim ersten `https`-
    /// Aufruf, statt einen Fehler zu liefern. Kompiliert hat das einwandfrei.
    ///
    /// Der Test verbindet sich auf einen Port, auf dem nichts lauscht: die
    /// TLS-Einrichtung passiert trotzdem, das Netz wird aber nicht berührt.
    /// Erwartet wird ein sauberer Fehlerschlüssel — kein Absturz.
    #[test]
    fn https_stuerzt_nicht_wegen_fehlendem_tls_anbieter_ab() {
        let ergebnis = hole("https://127.0.0.1:1/gibt-es-nicht");
        let fehler = ergebnis.expect_err("ein toter Port darf nicht gelingen");
        assert!(
            fehler == "update.error.offline" || fehler == "update.error.timeout",
            "unerwarteter Fehler: {fehler}"
        );
    }

    #[test]
    fn die_abfrage_geht_ausschliesslich_an_github() {
        assert!(API.starts_with("https://api.github.com/"));
        assert!(SEITE.starts_with("https://github.com/"));
        assert!(API.contains(REPO));
    }
}
