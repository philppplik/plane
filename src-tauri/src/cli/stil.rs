//! Farben und Umgebungsfragen der Konsolenausgabe.
//!
//! Farbe ist in Plane immer nur *Zusatz*: jede Information steht auch als
//! Symbol oder Wort da. Dadurch bleibt die Ausgabe in Pipes, Protokolldateien
//! und farblosen Terminals vollständig lesbar.

/// Markenakzent (`#A7EC5B`) als 24-Bit-ANSI-Sequenz.
pub const AKZENT: &str = "\x1b[38;2;167;236;91m";
/// Gedämpfter Text für Nebeninformationen.
pub const GEDAEMPFT: &str = "\x1b[2m";
/// Warnton.
pub const WARNUNG: &str = "\x1b[38;2;240;180;70m";
/// Fehlerton.
pub const FEHLER: &str = "\x1b[38;2;235;95;95m";
/// Fett.
pub const FETT: &str = "\x1b[1m";
/// Alles zurücksetzen.
pub const RESET: &str = "\x1b[0m";

/// Ausgabestil eines Laufs.
#[derive(Debug, Clone, Copy)]
pub struct Stil {
    farbig: bool,
}

impl Stil {
    /// Stil festlegen.
    ///
    /// `--no-color`, die Konvention `NO_COLOR` und eine umgeleitete Ausgabe
    /// schalten Farbe jeweils für sich allein ab – wer eine davon setzt, will
    /// keine Steuerzeichen in der Datei haben.
    pub fn neu(no_color_flag: bool, no_color_env: bool, ist_terminal: bool) -> Self {
        Self {
            farbig: !no_color_flag && !no_color_env && ist_terminal,
        }
    }

    /// Stil ohne jede Farbe – für JSON-Ausgabe und Tests.
    pub fn schlicht() -> Self {
        Self { farbig: false }
    }

    pub fn ist_farbig(&self) -> bool {
        self.farbig
    }

    /// Text einfärben. Ohne Farbunterstützung bleibt der Text unverändert –
    /// niemals entfällt der Text selbst.
    pub fn faerben(&self, code: &str, text: &str) -> String {
        if self.farbig {
            format!("{code}{text}{RESET}")
        } else {
            text.to_string()
        }
    }

    pub fn akzent(&self, text: &str) -> String {
        self.faerben(AKZENT, text)
    }

    pub fn gedaempft(&self, text: &str) -> String {
        self.faerben(GEDAEMPFT, text)
    }

    pub fn warnung(&self, text: &str) -> String {
        self.faerben(WARNUNG, text)
    }

    pub fn fehler(&self, text: &str) -> String {
        self.faerben(FEHLER, text)
    }

    pub fn ueberschrift(&self, text: &str) -> String {
        self.faerben(FETT, text)
    }
}

/// `true`, wenn die Umgebungsvariable `NO_COLOR` gesetzt ist.
pub fn no_color_gesetzt() -> bool {
    std::env::var_os("NO_COLOR").is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn farbe_nur_im_terminal_ohne_gegenteilige_wuensche() {
        assert!(Stil::neu(false, false, true).ist_farbig());
        assert!(!Stil::neu(true, false, true).ist_farbig());
        assert!(!Stil::neu(false, true, true).ist_farbig());
        assert!(!Stil::neu(false, false, false).ist_farbig());
    }

    #[test]
    fn ohne_farbe_bleibt_der_text_unveraendert() {
        let stil = Stil::schlicht();
        assert_eq!(stil.akzent("Plane"), "Plane");
        assert_eq!(stil.fehler("Fehler"), "Fehler");
        assert_eq!(stil.ueberschrift("Titel"), "Titel");
    }

    #[test]
    fn mit_farbe_wird_der_text_eingerahmt_und_zurueckgesetzt() {
        let stil = Stil::neu(false, false, true);
        let text = stil.akzent("Plane");
        assert!(text.starts_with(AKZENT));
        assert!(text.ends_with(RESET));
        assert!(text.contains("Plane"));
    }
}
