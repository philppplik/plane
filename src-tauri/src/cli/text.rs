//! Auflösung von Meldungen und Anzeigenamen.
//!
//! Die Engine liefert alle Texte als Übersetzungsschlüssel, teils mit
//! Argumenten hinter einem `|` (`"warn.process_running|chrome.exe"`). Der
//! Grund: die Engine kennt keine Sprache. Erst hier – an der Grenze zur
//! Darstellung – wird daraus ein lesbarer Satz. Alle Funktionen sind reine
//! Zeichenkettenlogik und dadurch ohne Terminal testbar.

use crate::engine::{Category, Risk, Target};
use crate::i18n;

/// Meldungsschlüssel der Engine in lesbaren Text übersetzen.
///
/// Warum eine eigene Funktion: `skip_reason`, `warnings` und `errors` haben
/// alle dasselbe Format `schlüssel|arg0|arg1`. Ohne zentrale Auflösung würde
/// jede Ausgabestelle das Trennzeichen neu behandeln – und eine davon
/// vergessen.
pub fn meldung(lang: &str, roh: &str) -> String {
    if roh.trim().is_empty() {
        return String::new();
    }
    let mut teile = roh.split('|');
    let schluessel = teile.next().unwrap_or("");
    let argumente: Vec<&str> = teile.collect();
    if argumente.is_empty() {
        i18n::t(lang, schluessel)
    } else {
        i18n::format(lang, schluessel, &argumente)
    }
}

/// Mehrere Meldungen auflösen. Leere Einträge fallen weg.
pub fn meldungen(lang: &str, rohe: &[String]) -> Vec<String> {
    rohe.iter()
        .map(|r| meldung(lang, r))
        .filter(|m| !m.is_empty())
        .collect()
}

/// Anzeigename eines Ziels. Unbekannte Schlüssel geben sich selbst zurück,
/// damit ein fehlender Katalogeintrag sichtbar bleibt statt zu verschwinden.
pub fn zielname(lang: &str, key: &str) -> String {
    i18n::t(lang, &format!("target.{key}.name"))
}

/// Beschreibung eines Ziels.
pub fn zielbeschreibung(lang: &str, key: &str) -> String {
    i18n::t(lang, &format!("target.{key}.description"))
}

/// Anzeigename einer Kategorie.
pub fn kategoriename(lang: &str, kategorie: Category) -> String {
    i18n::t(lang, &kategorie.i18n_key())
}

/// Anzeigename einer Risikostufe.
pub fn risikoname(lang: &str, risiko: Risk) -> String {
    i18n::t(lang, &format!("risk.{}", risiko.key()))
}

/// Erläuterung zur Risikostufe – der Satz, der im Detailbereich steht.
pub fn risikohinweis(lang: &str, risiko: Risk) -> String {
    i18n::t(lang, &format!("risk.{}.hint", risiko.key()))
}

/// Reines ASCII-Symbol je Risikostufe.
///
/// Farbe allein reicht nicht: in Pipes, Protokolldateien und Terminals ohne
/// Farbunterstützung muss die Stufe trotzdem erkennbar sein.
pub fn risikosymbol(risiko: Risk) -> &'static str {
    match risiko {
        Risk::Safe => "+",
        Risk::Notice => "!",
        Risk::Caution => "#",
    }
}

/// Statuszeichen für eine Zeile des Ergebnisberichts.
pub fn statuszeichen(ok: bool, uebersprungen: bool) -> &'static str {
    if uebersprungen {
        "-"
    } else if ok {
        "+"
    } else {
        "x"
    }
}

/// Kurze Kennzeichnung besonderer Eigenschaften eines Ziels
/// (Administratorrechte, reiner Vorschlag). Leer, wenn nichts zutrifft.
pub fn zielmerkmale(lang: &str, ziel: &Target) -> Vec<String> {
    let mut merkmale = Vec::new();
    if ziel.requires_admin {
        merkmale.push(i18n::t(lang, "risk.needs_admin"));
    }
    if ziel.is_suggestion_only() {
        merkmale.push(i18n::t(lang, "risk.suggestion_only"));
    }
    merkmale
}

/// Zeichenkette auf eine Höchstlänge kürzen und mit `…` kennzeichnen.
///
/// Arbeitet auf `char`-Ebene, nicht auf Bytes – sonst zerschneidet ein
/// gekürzter Umlaut die Ausgabe.
pub fn kuerzen(text: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let zeichen: Vec<char> = text.chars().collect();
    if zeichen.len() <= max {
        return text.to_string();
    }
    if max == 1 {
        return "…".to_string();
    }
    let mut gekuerzt: String = zeichen[..max - 1].iter().collect();
    gekuerzt.push('…');
    gekuerzt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meldung_ohne_argument_wird_uebersetzt() {
        let text = meldung("de", "skip.needs_admin");
        assert!(text.contains("Administratorrechte"));
        assert_ne!(text, "skip.needs_admin");
    }

    #[test]
    fn meldung_mit_argument_setzt_den_platzhalter_ein() {
        let text = meldung("de", "warn.process_running|chrome.exe");
        assert!(text.starts_with("chrome.exe"));
        assert!(!text.contains("{0}"));
    }

    #[test]
    fn meldung_mit_mehreren_argumenten_fuellt_alle_platzhalter() {
        let text = meldung("en", "clean.summary|1.2 GB|42");
        assert_eq!(text, "1.2 GB freed, 42 items removed");
    }

    #[test]
    fn leere_meldung_bleibt_leer() {
        assert_eq!(meldung("de", ""), "");
        assert_eq!(meldung("de", "   "), "");
    }

    #[test]
    fn unbekannter_meldungsschluessel_bleibt_sichtbar() {
        assert_eq!(meldung("de", "gibt.es.nicht"), "gibt.es.nicht");
    }

    #[test]
    fn meldungsliste_filtert_leere_eintraege() {
        let liste = vec![String::new(), "skip.needs_admin".to_string()];
        assert_eq!(meldungen("de", &liste).len(), 1);
    }

    #[test]
    fn sprachumschaltung_aendert_den_text() {
        let de = meldung("de", "skip.needs_admin");
        let en = meldung("en", "skip.needs_admin");
        assert_ne!(de, en);
        assert!(en.contains("administrator"));
    }

    #[test]
    fn zielname_nutzt_den_katalogschluessel() {
        assert_eq!(
            zielname("de", "system.temp.user"),
            "Temporäre Dateien".to_string()
        );
    }

    #[test]
    fn risikosymbole_sind_paarweise_verschieden() {
        assert_ne!(risikosymbol(Risk::Safe), risikosymbol(Risk::Notice));
        assert_ne!(risikosymbol(Risk::Notice), risikosymbol(Risk::Caution));
    }

    #[test]
    fn statuszeichen_unterscheidet_die_drei_faelle() {
        assert_eq!(statuszeichen(true, false), "+");
        assert_eq!(statuszeichen(false, false), "x");
        assert_eq!(statuszeichen(true, true), "-");
    }

    #[test]
    fn kuerzen_laesst_kurze_texte_unberuehrt() {
        assert_eq!(kuerzen("abc", 5), "abc");
        assert_eq!(kuerzen("abcde", 5), "abcde");
    }

    #[test]
    fn kuerzen_haengt_auslassungszeichen_an() {
        assert_eq!(kuerzen("abcdef", 4), "abc…");
        assert_eq!(kuerzen("abc", 1), "…");
        assert_eq!(kuerzen("abc", 0), "");
    }

    #[test]
    fn kuerzen_zerschneidet_keine_umlaute() {
        // Bei Byte-Arithmetik würde hier ein halbes Zeichen entstehen.
        let gekuerzt = kuerzen("Temporäre Dateien", 6);
        assert_eq!(gekuerzt.chars().count(), 6);
        assert!(gekuerzt.starts_with("Tempo"));
    }
}
