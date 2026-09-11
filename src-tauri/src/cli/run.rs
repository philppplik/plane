//! Ausführung der Unterbefehle.
//!
//! Hier – und nur hier – wird tatsächlich auf `stdout`/`stderr` geschrieben.
//! Alles, was sich sinnvoll prüfen lässt (Auswahl, Aufbereitung, Exitcodes),
//! liegt in den Nachbarmodulen und wird von hier nur noch zusammengesetzt.

use std::io::{IsTerminal, Write};
use std::path::PathBuf;

use crate::engine::fsutil::format_bytes;
use crate::engine::{
    self, CancelToken, CleanReport, CleanRequest, Progress, RunContext, ScanReport,
};
use crate::i18n;

use super::args::{self, Bedienfehler, Befehl, Cli, Exitcode};
use super::info;
use super::progress;
use super::stil::Stil;
use super::table;
use super::text;
use super::tui;

/// Ausgabekanal eines Laufs.
///
/// Fasst zusammen, was fast jede Ausgabefunktion braucht: Sprache, Farbe,
/// Breite und ob überhaupt ein Mensch zuschaut.
#[derive(Debug, Clone)]
pub struct Ausgabe {
    pub lang: String,
    pub stil: Stil,
    pub breite: usize,
    pub interaktiv: bool,
    pub json: bool,
}

impl Ausgabe {
    /// Ausgabekanal aus den globalen Optionen ableiten.
    pub fn neu(lang: String, no_color: bool, json: bool) -> Self {
        let ist_terminal = std::io::stdout().is_terminal();
        Self {
            lang,
            stil: if json {
                Stil::schlicht()
            } else {
                Stil::neu(no_color, super::stil::no_color_gesetzt(), ist_terminal)
            },
            breite: terminalbreite(),
            // Ohne Terminal darf nichts nachfragen – sonst hängt jede Pipeline.
            interaktiv: ist_terminal && std::io::stdin().is_terminal() && !json,
            json,
        }
    }

    fn zeile(&self, text: &str) {
        println!("{text}");
    }

    fn tabelle(&self, tabelle: &table::Tabelle) {
        for (index, zeile) in tabelle.rendern().iter().enumerate() {
            if index == 0 {
                self.zeile(&self.stil.ueberschrift(zeile));
            } else if index == 1 {
                self.zeile(&self.stil.gedaempft(zeile));
            } else {
                self.zeile(zeile);
            }
        }
    }
}

/// Terminalbreite ermitteln, mit vernünftigem Rückfallwert.
fn terminalbreite() -> usize {
    crossterm::terminal::size()
        .map(|(spalten, _)| spalten as usize)
        .unwrap_or(100)
        .clamp(40, 200)
}

/// Ablageort der Registry-Sicherungen.
///
/// Vor jeder Registry-Änderung legt die Engine dort eine `.reg`-Datei ab. Der
/// Ordner liegt im Benutzerprofil, damit die Sicherung auch ohne
/// Administratorrechte geschrieben werden kann.
pub fn sicherungsordner() -> PathBuf {
    let basis = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    crate::state::backup_path(&basis.join("Plane"))
}

/// Gesamten Lauf ausführen und den Exitcode zurückgeben.
pub fn ausfuehren(cli: Cli) -> Exitcode {
    let lang =
        args::sprache_bestimmen(cli.lang.as_deref(), args::sprache_aus_umgebung().as_deref());

    match cli.command {
        Some(Befehl::List { category, json }) => {
            let ausgabe = Ausgabe::neu(lang, cli.no_color, json);
            melde(liste(&ausgabe, category.as_deref()), &ausgabe)
        }
        Some(Befehl::Scan { targets, json, all }) => {
            let ausgabe = Ausgabe::neu(lang, cli.no_color, json);
            melde(analysieren(&ausgabe, &targets, all), &ausgabe)
        }
        Some(Befehl::Clean {
            targets,
            dry_run,
            yes,
            json,
        }) => {
            let ausgabe = Ausgabe::neu(lang, cli.no_color, json);
            melde(bereinigen(&ausgabe, &targets, dry_run, yes), &ausgabe)
        }
        Some(Befehl::Info { json }) => {
            let ausgabe = Ausgabe::neu(lang, cli.no_color, json);
            melde(systeminfo(&ausgabe), &ausgabe)
        }
        Some(Befehl::Tui) => starte_tui(&lang, cli.no_color),
        // Ohne Unterbefehl ist die Oberfläche gemeint – aber nur, wenn
        // jemand zuschaut. In einer Pipeline wäre das eine Falle.
        None => {
            if std::io::stdout().is_terminal() {
                starte_tui(&lang, cli.no_color)
            } else {
                eprintln!("plane-cli: kein Unterbefehl angegeben (`plane-cli --help`)");
                Exitcode::Bedienfehler
            }
        }
    }
}

/// Ergebnis eines Unterbefehls in einen Exitcode überführen und Fehler melden.
fn melde(ergebnis: Result<Exitcode, Bedienfehler>, ausgabe: &Ausgabe) -> Exitcode {
    match ergebnis {
        Ok(code) => code,
        Err(fehler) => {
            eprintln!("{}", ausgabe.stil.fehler(&format!("Fehler: {fehler}")));
            Exitcode::Bedienfehler
        }
    }
}

// ---------------------------------------------------------------------------
// list
// ---------------------------------------------------------------------------

fn liste(ausgabe: &Ausgabe, kategorie: Option<&str>) -> Result<Exitcode, Bedienfehler> {
    let kategorie = kategorie.map(args::kategorie_aus_name).transpose()?;

    if ausgabe.json {
        let ziele: Vec<serde_json::Value> = engine::TARGETS
            .iter()
            .filter(|z| kategorie.is_none_or(|k| k == z.category))
            .map(|z| {
                serde_json::json!({
                    "key": z.key,
                    "category": z.category,
                    "risk": z.risk,
                    "requires_admin": z.requires_admin,
                    "default_enabled": z.default_enabled,
                    "suggestion_only": z.is_suggestion_only(),
                    "name": text::zielname(&ausgabe.lang, z.key),
                    "description": text::zielbeschreibung(&ausgabe.lang, z.key),
                })
            })
            .collect();
        println!("{}", serde_json::json!({ "targets": ziele }));
        return Ok(Exitcode::Erfolg);
    }

    ausgabe.tabelle(&table::zielliste(&ausgabe.lang, kategorie));
    Ok(Exitcode::Erfolg)
}

// ---------------------------------------------------------------------------
// scan
// ---------------------------------------------------------------------------

fn analysieren(ausgabe: &Ausgabe, ziele: &[String], alle: bool) -> Result<Exitcode, Bedienfehler> {
    let auswahl = args::scanauswahl(ziele)?;
    let bericht = scan_mit_fortschritt(ausgabe, &auswahl);

    if ausgabe.json {
        println!("{}", serde_json::to_string(&bericht).unwrap_or_default());
        return Ok(args::exitcode_fuer_scan(&bericht));
    }

    scan_ausgeben(ausgabe, &bericht, alle);
    Ok(args::exitcode_fuer_scan(&bericht))
}

/// Analyse mit Fortschrittsanzeige und Abbruchmöglichkeit.
fn scan_mit_fortschritt(ausgabe: &Ausgabe, auswahl: &[String]) -> ScanReport {
    let abbruch = abbruch_token();
    let ctx = RunContext::new().with_cancel(abbruch);
    let ctx = fortschritt_anhaengen(ctx, ausgabe);
    let bericht = engine::scan(auswahl, &ctx);
    fortschritt_abschliessen(ausgabe);
    bericht
}

fn scan_ausgeben(ausgabe: &Ausgabe, bericht: &ScanReport, alle: bool) {
    let tabelle = table::scan_tabelle(&ausgabe.lang, bericht, alle);
    if tabelle.zeilenzahl() == 0 {
        ausgabe.zeile(&i18n::t(&ausgabe.lang, "home.nothing_found"));
    } else {
        ausgabe.tabelle(&tabelle);
        ausgabe.zeile("");
        ausgabe.tabelle(&table::kategorie_tabelle(&ausgabe.lang, bericht));
    }
    ausgabe.zeile("");
    ausgabe.zeile(
        &ausgabe
            .stil
            .akzent(&table::scan_zusammenfassung(&ausgabe.lang, bericht)),
    );
    if bericht.cancelled {
        ausgabe.zeile(&ausgabe.stil.warnung(&i18n::t(&ausgabe.lang, "cli.aborted")));
    }
}

// ---------------------------------------------------------------------------
// clean
// ---------------------------------------------------------------------------

fn bereinigen(
    ausgabe: &Ausgabe,
    ziele: &[String],
    trockenlauf: bool,
    ohne_rueckfrage: bool,
) -> Result<Exitcode, Bedienfehler> {
    let auswahl = args::cleanauswahl(ziele)?;
    if auswahl.is_empty() {
        ausgabe.zeile(&i18n::t(&ausgabe.lang, "cli.no_selection"));
        return Ok(Exitcode::Erfolg);
    }

    // Ohne `--yes` zeigt Plane erst, worum es geht. Wer blind löscht, soll das
    // ausdrücklich verlangen müssen.
    if !ohne_rueckfrage && !ausgabe.json {
        if !ausgabe.interaktiv {
            return Err(Bedienfehler(
                "Rückfrage nicht möglich (keine Konsole). Bitte `--yes` angeben.".into(),
            ));
        }
        let vorschau = scan_mit_fortschritt(ausgabe, &auswahl);
        scan_ausgeben(ausgabe, &vorschau, false);
        ausgabe.zeile("");
        if !bestaetigen(ausgabe, &auswahl)? {
            ausgabe.zeile(&i18n::t(&ausgabe.lang, "cli.aborted"));
            return Ok(Exitcode::Abgebrochen);
        }
    }

    let anfrage = CleanRequest {
        targets: auswahl,
        only_paths: Vec::new(),
        dry_run: trockenlauf,
    };

    let abbruch = abbruch_token();
    let ctx = RunContext::new().with_cancel(abbruch);
    let ctx = fortschritt_anhaengen(ctx, ausgabe);
    let bericht = engine::clean(&anfrage, &sicherungsordner(), &ctx);
    fortschritt_abschliessen(ausgabe);

    if ausgabe.json {
        println!("{}", serde_json::to_string(&bericht).unwrap_or_default());
    } else {
        clean_ausgeben(ausgabe, &bericht, trockenlauf);
    }
    Ok(args::exitcode_fuer_clean(&bericht))
}

fn clean_ausgeben(ausgabe: &Ausgabe, bericht: &CleanReport, trockenlauf: bool) {
    ausgabe.tabelle(&table::clean_tabelle(&ausgabe.lang, bericht));
    ausgabe.zeile("");
    ausgabe.zeile(&ausgabe.stil.akzent(&table::clean_zusammenfassung(
        &ausgabe.lang,
        bericht,
        trockenlauf,
    )));
    if let Some(pfad) = &bericht.registry_backup {
        ausgabe.zeile(&i18n::format(
            &ausgabe.lang,
            "clean.registry_backup",
            &[pfad],
        ));
    }
    if !bericht.error.is_empty() {
        ausgabe.zeile(
            &ausgabe
                .stil
                .fehler(&text::meldung(&ausgabe.lang, &bericht.error)),
        );
    }
    if bericht.cancelled {
        ausgabe.zeile(&ausgabe.stil.warnung(&i18n::t(&ausgabe.lang, "cli.aborted")));
    }
}

/// Rückfrage vor dem Löschen.
///
/// Riskante Ziele werden **einzeln** benannt und einzeln bestätigt: bei einer
/// Sammelfrage übersieht man genau den einen Punkt, der wehtut.
fn bestaetigen(ausgabe: &Ausgabe, auswahl: &[String]) -> Result<bool, Bedienfehler> {
    let riskant = table::riskante_ziele(auswahl);
    if !riskant.is_empty() {
        ausgabe.zeile(
            &ausgabe
                .stil
                .warnung(&i18n::t(&ausgabe.lang, "confirm.title")),
        );
        ausgabe.zeile(&i18n::t(&ausgabe.lang, "confirm.warning"));
        for key in riskant {
            let frage = format!(
                "  {} {} — {} ",
                text::risikosymbol(engine::Risk::Caution),
                text::zielname(&ausgabe.lang, key),
                i18n::t(&ausgabe.lang, "cli.confirm_prompt").trim()
            );
            if !frage_stellen(ausgabe, &frage)? {
                return Ok(false);
            }
        }
    }
    frage_stellen(ausgabe, &i18n::t(&ausgabe.lang, "cli.confirm_prompt"))
}

fn frage_stellen(ausgabe: &Ausgabe, frage: &str) -> Result<bool, Bedienfehler> {
    print!("{frage}");
    std::io::stdout()
        .flush()
        .map_err(|e| Bedienfehler(e.to_string()))?;
    let mut eingabe = String::new();
    if std::io::stdin().read_line(&mut eingabe).is_err() {
        return Ok(false);
    }
    Ok(antwort_ist_ja(&ausgabe.lang, &eingabe))
}

/// Prüft, ob eine Eingabe als Zustimmung gilt.
///
/// Neben dem sprachabhängigen Kürzel (`cli.confirm_yes`) werden die
/// ausgeschriebenen Formen akzeptiert. Alles andere – auch eine leere
/// Eingabe – bedeutet Nein.
pub fn antwort_ist_ja(lang: &str, eingabe: &str) -> bool {
    let antwort = eingabe.trim().to_lowercase();
    if antwort.is_empty() {
        return false;
    }
    let kuerzel = i18n::t(lang, "cli.confirm_yes").to_lowercase();
    antwort == kuerzel || matches!(antwort.as_str(), "y" | "yes" | "j" | "ja")
}

// ---------------------------------------------------------------------------
// info
// ---------------------------------------------------------------------------

fn systeminfo(ausgabe: &Ausgabe) -> Result<Exitcode, Bedienfehler> {
    let bericht = info::erheben();

    if ausgabe.json {
        println!("{}", serde_json::to_string(&bericht).unwrap_or_default());
        return Ok(Exitcode::Erfolg);
    }

    ausgabe.zeile(&ausgabe.stil.akzent(&format!(
        "Plane {} — {}",
        env!("CARGO_PKG_VERSION"),
        i18n::t(&ausgabe.lang, "app.tagline")
    )));
    ausgabe.zeile("");

    let mut tabelle = table::Tabelle::neu(
        &["", ""],
        &[table::Ausrichtung::Links, table::Ausrichtung::Links],
    );
    tabelle.zeile(vec!["OS".into(), bericht.os.clone()]);
    tabelle.zeile(vec!["CPU".into(), bericht.cpu.clone()]);
    tabelle.zeile(vec!["Arch".into(), bericht.arch.clone()]);
    tabelle.zeile(vec!["RAM".into(), format_bytes(bericht.ram)]);
    tabelle.zeile(vec![
        i18n::t(&ausgabe.lang, "about.system"),
        i18n::t(
            &ausgabe.lang,
            if bericht.is_admin {
                "about.admin_yes"
            } else {
                "about.admin_no"
            },
        ),
    ]);
    tabelle.zeile(vec![
        "Ziele".into(),
        format!(
            "{} ({} empfohlen)",
            bericht.target_count, bericht.default_selection
        ),
    ]);
    ausgabe.tabelle(&tabelle);
    ausgabe.zeile("");

    let mut laufwerke = table::Tabelle::neu(
        &["LAUFWERK", "GESAMT", "FREI", "BELEGT"],
        &[
            table::Ausrichtung::Links,
            table::Ausrichtung::Rechts,
            table::Ausrichtung::Rechts,
            table::Ausrichtung::Rechts,
        ],
    );
    for laufwerk in &bericht.disks {
        let markierung = if laufwerk.mount.starts_with(&bericht.system_drive) {
            format!("{} *", laufwerk.mount)
        } else {
            laufwerk.mount.clone()
        };
        laufwerke.zeile(vec![
            markierung,
            format_bytes(laufwerk.total),
            format_bytes(laufwerk.free),
            format!("{:.1} %", laufwerk.used_percent),
        ]);
    }
    ausgabe.tabelle(&laufwerke);
    Ok(Exitcode::Erfolg)
}

// ---------------------------------------------------------------------------
// tui
// ---------------------------------------------------------------------------

fn starte_tui(lang: &str, no_color: bool) -> Exitcode {
    match tui::starten(lang, no_color) {
        Ok(code) => code,
        Err(fehler) => {
            eprintln!("plane-cli: {fehler}");
            Exitcode::Fehlgeschlagen
        }
    }
}

// ---------------------------------------------------------------------------
// Fortschritt und Abbruch
// ---------------------------------------------------------------------------

/// Abbruchsignal anlegen und mit Strg+C verbinden.
///
/// Der Handler darf pro Prozess nur einmal gesetzt werden; scheitert das
/// (z. B. weil bereits ein Handler läuft), bleibt der Lauf trotzdem gültig –
/// er ist dann nur nicht abbrechbar.
fn abbruch_token() -> CancelToken {
    let token = CancelToken::new();
    let kopie = token.clone();
    let _ = ctrlc::set_handler(move || kopie.cancel());
    token
}

/// Fortschrittsausgabe anhängen, sofern sie erlaubt ist.
///
/// Bei `--json` oder umgeleiteter Ausgabe bleibt der Kontext ohne Sink: eine
/// Fortschrittszeile würde die Pipe mit Steuerzeichen verunreinigen.
fn fortschritt_anhaengen(ctx: RunContext, ausgabe: &Ausgabe) -> RunContext {
    if !fortschritt_erlaubt(ausgabe) {
        return ctx;
    }
    let lang = ausgabe.lang.clone();
    let breite = ausgabe.breite;
    ctx.with_progress(Box::new(move |p: Progress| {
        let zeile = progress::zeile(&lang, &p, breite.saturating_sub(1));
        print!("{}", progress::ueberschreibend(&zeile, breite - 1));
        let _ = std::io::stdout().flush();
    }))
}

/// `true`, wenn eine Fortschrittszeile geschrieben werden darf.
pub fn fortschritt_erlaubt(ausgabe: &Ausgabe) -> bool {
    !ausgabe.json && std::io::stdout().is_terminal()
}

/// Fortschrittszeile abräumen, damit die Ergebnistabelle sauber beginnt.
fn fortschritt_abschliessen(ausgabe: &Ausgabe) {
    if fortschritt_erlaubt(ausgabe) {
        print!("{}", progress::ueberschreibend("", ausgabe.breite - 1));
        print!("\r");
        let _ = std::io::stdout().flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ausgabe(json: bool) -> Ausgabe {
        Ausgabe {
            lang: "de".into(),
            stil: Stil::schlicht(),
            breite: 80,
            interaktiv: false,
            json,
        }
    }

    #[test]
    fn zustimmung_akzeptiert_das_sprachkuerzel() {
        assert!(antwort_ist_ja("de", "j"));
        assert!(antwort_ist_ja("de", "ja\n"));
        assert!(antwort_ist_ja("en", "y"));
        assert!(antwort_ist_ja("en", " YES \r\n"));
    }

    #[test]
    fn leere_eingabe_bedeutet_nein() {
        assert!(!antwort_ist_ja("de", ""));
        assert!(!antwort_ist_ja("de", "\n"));
        assert!(!antwort_ist_ja("de", "   "));
    }

    #[test]
    fn alles_andere_bedeutet_nein() {
        assert!(!antwort_ist_ja("de", "n"));
        assert!(!antwort_ist_ja("de", "nein"));
        assert!(!antwort_ist_ja("de", "vielleicht"));
    }

    #[test]
    fn json_unterdrueckt_die_fortschrittszeile() {
        // Auch in einem Terminal darf JSON nie durch \r-Zeilen verschmutzt werden.
        assert!(!fortschritt_erlaubt(&ausgabe(true)));
    }

    #[test]
    fn json_ausgabe_ist_nie_farbig() {
        let a = Ausgabe::neu("de".into(), false, true);
        assert!(!a.stil.ist_farbig());
        assert!(!a.interaktiv);
    }

    #[test]
    fn sicherungsordner_liegt_im_benutzerprofil() {
        let pfad = sicherungsordner();
        assert!(pfad.to_string_lossy().contains("Plane"));
        assert!(pfad.is_absolute() || cfg!(not(windows)));
    }

    #[test]
    fn terminalbreite_bleibt_in_sinnvollen_grenzen() {
        let breite = terminalbreite();
        assert!((40..=200).contains(&breite));
    }
}
