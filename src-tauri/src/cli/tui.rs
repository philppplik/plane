//! Textoberfläche (TUI).
//!
//! Dieselbe Engine wie GUI und CLI, nur im Terminal. Der Aufbau folgt dem
//! Ablauf der grafischen Fassung: erst analysieren, dann auswählen, dann
//! bereinigen – es wird nie ungefragt gelöscht.
//!
//! Die **Zustandsmaschine** ([`Oberflaeche`]) ist vom Zeichnen getrennt, damit
//! sich Navigation und Auswahl ohne Terminal testen lassen. Das Zeichnen
//! selbst ist reine Darstellung ohne Logik.

use std::io::{self, Stdout};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{Frame, Terminal};

use crate::engine::fsutil::format_bytes;
use crate::engine::{
    self, CancelToken, CleanReport, CleanRequest, Progress, RunContext, ScanReport,
};
use crate::i18n;

use super::args::Exitcode;
use super::text;

/// Kleinste sinnvolle Terminalgröße. Darunter wird nur ein Hinweis gezeigt.
const MIN_BREITE: u16 = 80;
const MIN_HOEHE: u16 = 20;

/// Markenfarbe `#A7EC5B`.
const AKZENT: Color = Color::Rgb(167, 236, 91);

/// ASCII-Logo für den Startbildschirm.
const LOGO: &[&str] = &[
    r"  ██████╗ ██╗      █████╗ ███╗   ██╗███████╗",
    r"  ██╔══██╗██║     ██╔══██╗████╗  ██║██╔════╝",
    r"  ██████╔╝██║     ███████║██╔██╗ ██║█████╗  ",
    r"  ██╔═══╝ ██║     ██╔══██║██║╚██╗██║██╔══╝  ",
    r"  ██║     ███████╗██║  ██║██║ ╚████║███████╗",
    r"  ╚═╝     ╚══════╝╚═╝  ╚═╝╚═╝  ╚═══╝╚══════╝",
];

/// Welcher Bereich die Tastatureingaben bekommt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spalte {
    Kategorien,
    Ziele,
}

/// Was gerade angezeigt wird.
#[derive(Debug, Clone, PartialEq)]
pub enum Ansicht {
    /// Startbildschirm mit Logo, vor der ersten Analyse.
    Start,
    /// Auswahl von Kategorien und Zielen.
    Auswahl,
    /// Analyse oder Bereinigung läuft.
    Laeuft { phase: &'static str },
    /// Ergebnis der Bereinigung.
    Ergebnis,
    /// Tastaturhilfe.
    Hilfe,
}

/// Ergebnis einer Tasteneingabe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aktion {
    /// Nichts weiter zu tun.
    Nichts,
    /// Analyse starten.
    Analysieren,
    /// Bereinigung starten.
    Bereinigen,
    /// Laufenden Vorgang abbrechen.
    Abbrechen,
    /// Oberfläche beenden.
    Beenden,
}

/// Zustand der Oberfläche.
///
/// Enthält bewusst **keine** Terminal- oder Zeichenlogik, damit die
/// Navigation testbar bleibt.
pub struct Oberflaeche {
    pub lang: String,
    pub ansicht: Ansicht,
    pub spalte: Spalte,
    /// Kategorien in Anzeigereihenfolge.
    pub kategorien: Vec<engine::Category>,
    pub kategorie_index: usize,
    pub ziel_index: usize,
    /// Ausgewählte Zielschlüssel.
    pub auswahl: Vec<String>,
    /// Letztes Analyseergebnis.
    pub scan: Option<ScanReport>,
    /// Letztes Bereinigungsergebnis.
    pub clean: Option<CleanReport>,
    /// Nur simulieren.
    pub trockenlauf: bool,
    /// Aktueller Fortschritt.
    pub fortschritt: Option<Progress>,
    /// Meldung in der Fußzeile.
    pub hinweis: String,
}

impl Oberflaeche {
    pub fn neu(lang: &str) -> Self {
        Self {
            lang: lang.to_string(),
            ansicht: Ansicht::Start,
            spalte: Spalte::Kategorien,
            kategorien: engine::Category::ALL.to_vec(),
            kategorie_index: 0,
            ziel_index: 0,
            auswahl: engine::default_selection()
                .iter()
                .map(|s| s.to_string())
                .collect(),
            scan: None,
            clean: None,
            trockenlauf: false,
            fortschritt: None,
            hinweis: String::new(),
        }
    }

    /// Aktuell gewählte Kategorie.
    pub fn kategorie(&self) -> engine::Category {
        self.kategorien[self.kategorie_index.min(self.kategorien.len() - 1)]
    }

    /// Ziele der aktuellen Kategorie.
    pub fn ziele(&self) -> Vec<&'static engine::Target> {
        engine::targets_in(self.kategorie()).collect()
    }

    /// Aktuell markiertes Ziel.
    pub fn aktuelles_ziel(&self) -> Option<&'static engine::Target> {
        let ziele = self.ziele();
        ziele
            .get(self.ziel_index.min(ziele.len().saturating_sub(1)))
            .copied()
    }

    /// Analyseergebnis zu einem Zielschlüssel.
    pub fn scan_fuer(&self, key: &str) -> Option<&engine::TargetScan> {
        self.scan.as_ref()?.targets.iter().find(|t| t.key == key)
    }

    /// Summe der ausgewählten Ziele in Bytes.
    pub fn auswahl_groesse(&self) -> u64 {
        let Some(bericht) = &self.scan else {
            return 0;
        };
        bericht
            .targets
            .iter()
            .filter(|t| self.auswahl.contains(&t.key) && !t.skipped)
            .map(|t| t.size)
            .sum()
    }

    /// Gefundene Größe einer Kategorie.
    pub fn kategorie_groesse(&self, kategorie: engine::Category) -> u64 {
        let Some(bericht) = &self.scan else {
            return 0;
        };
        bericht
            .targets
            .iter()
            .filter(|t| t.category == kategorie)
            .map(|t| t.size)
            .sum()
    }

    pub fn ist_ausgewaehlt(&self, key: &str) -> bool {
        self.auswahl.iter().any(|k| k == key)
    }

    /// Auswahl eines Ziels umschalten.
    pub fn umschalten(&mut self, key: &str) {
        if let Some(index) = self.auswahl.iter().position(|k| k == key) {
            self.auswahl.remove(index);
        } else {
            self.auswahl.push(key.to_string());
        }
    }

    /// Alle Ziele auswählen.
    pub fn alle_waehlen(&mut self) {
        self.auswahl = engine::TARGETS.iter().map(|t| t.key.to_string()).collect();
    }

    /// Auswahl leeren.
    pub fn nichts_waehlen(&mut self) {
        self.auswahl.clear();
    }

    /// Auf die empfohlene Auswahl zurücksetzen.
    pub fn empfohlen_waehlen(&mut self) {
        self.auswahl = engine::default_selection()
            .iter()
            .map(|s| s.to_string())
            .collect();
    }

    /// Sprache umschalten.
    pub fn sprache_umschalten(&mut self) {
        self.lang = if self.lang == "de" {
            "en".to_string()
        } else {
            "de".to_string()
        };
    }

    fn nach_unten(&mut self) {
        match self.spalte {
            Spalte::Kategorien => {
                if self.kategorie_index + 1 < self.kategorien.len() {
                    self.kategorie_index += 1;
                    self.ziel_index = 0;
                }
            }
            Spalte::Ziele => {
                let anzahl = self.ziele().len();
                if anzahl > 0 && self.ziel_index + 1 < anzahl {
                    self.ziel_index += 1;
                }
            }
        }
    }

    fn nach_oben(&mut self) {
        match self.spalte {
            Spalte::Kategorien => {
                if self.kategorie_index > 0 {
                    self.kategorie_index -= 1;
                    self.ziel_index = 0;
                }
            }
            Spalte::Ziele => {
                self.ziel_index = self.ziel_index.saturating_sub(1);
            }
        }
    }

    /// Eine Taste verarbeiten.
    ///
    /// Gibt zurück, was der Aufrufer anstoßen soll. Der Zustand wird dabei
    /// bereits aktualisiert.
    pub fn taste(&mut self, taste: KeyCode, modifikatoren: KeyModifiers) -> Aktion {
        // Strg+C beendet immer.
        if modifikatoren.contains(KeyModifiers::CONTROL) && taste == KeyCode::Char('c') {
            return if matches!(self.ansicht, Ansicht::Laeuft { .. }) {
                Aktion::Abbrechen
            } else {
                Aktion::Beenden
            };
        }

        // Während eines Laufs sind nur Abbruch und Hilfe erlaubt.
        if let Ansicht::Laeuft { .. } = self.ansicht {
            return match taste {
                KeyCode::Esc | KeyCode::Char('q') => Aktion::Abbrechen,
                _ => Aktion::Nichts,
            };
        }

        if self.ansicht == Ansicht::Hilfe {
            self.ansicht = if self.scan.is_some() {
                Ansicht::Auswahl
            } else {
                Ansicht::Start
            };
            return Aktion::Nichts;
        }

        match taste {
            KeyCode::Char('q') | KeyCode::Esc => Aktion::Beenden,
            KeyCode::Char('?') | KeyCode::F(1) => {
                self.ansicht = Ansicht::Hilfe;
                Aktion::Nichts
            }
            KeyCode::Char('s') | KeyCode::Enter if self.ansicht == Ansicht::Start => {
                Aktion::Analysieren
            }
            KeyCode::Char('s') => Aktion::Analysieren,
            KeyCode::Char('c') => Aktion::Bereinigen,
            KeyCode::Char('d') => {
                self.trockenlauf = !self.trockenlauf;
                Aktion::Nichts
            }
            KeyCode::Char('L') => {
                self.sprache_umschalten();
                Aktion::Nichts
            }
            KeyCode::Char('a') => {
                self.alle_waehlen();
                Aktion::Nichts
            }
            KeyCode::Char('n') => {
                self.nichts_waehlen();
                Aktion::Nichts
            }
            KeyCode::Char('r') => {
                self.empfohlen_waehlen();
                Aktion::Nichts
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.nach_unten();
                Aktion::Nichts
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.nach_oben();
                Aktion::Nichts
            }
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab => {
                self.spalte = Spalte::Ziele;
                Aktion::Nichts
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.spalte = Spalte::Kategorien;
                Aktion::Nichts
            }
            KeyCode::Char(' ') => {
                if self.spalte == Spalte::Ziele {
                    if let Some(ziel) = self.aktuelles_ziel() {
                        let key = ziel.key.to_string();
                        self.umschalten(&key);
                    }
                } else {
                    // In der Kategoriespalte schaltet Leertaste die ganze
                    // Kategorie um – alles oder nichts.
                    let kategorie = self.kategorie();
                    let keys: Vec<String> = engine::targets_in(kategorie)
                        .map(|t| t.key.to_string())
                        .collect();
                    let alle_drin = keys.iter().all(|k| self.ist_ausgewaehlt(k));
                    for key in keys {
                        if alle_drin {
                            if let Some(i) = self.auswahl.iter().position(|k| *k == key) {
                                self.auswahl.remove(i);
                            }
                        } else if !self.ist_ausgewaehlt(&key) {
                            self.auswahl.push(key);
                        }
                    }
                }
                Aktion::Nichts
            }
            _ => Aktion::Nichts,
        }
    }
}

// ---------------------------------------------------------------------------
// Ablauf
// ---------------------------------------------------------------------------

/// Nachricht aus dem Arbeitsthread.
enum Meldung {
    Fortschritt(Box<Progress>),
    ScanFertig(Box<ScanReport>),
    CleanFertig(Box<CleanReport>),
}

/// Die Oberfläche starten.
///
/// Setzt das Terminal in jedem Fall zurück – auch bei einer Panik, dafür sorgt
/// ein eigener Panic-Hook. Ein Absturz darf kein zerschossenes Terminal
/// hinterlassen.
pub fn starten(lang: &str, _no_color: bool) -> Result<Exitcode, String> {
    let (breite, hoehe) = crossterm::terminal::size().unwrap_or((MIN_BREITE, MIN_HOEHE));
    if breite < MIN_BREITE || hoehe < MIN_HOEHE {
        return Err(format!(
            "Das Terminal ist zu klein ({breite}×{hoehe}). Plane braucht mindestens {MIN_BREITE}×{MIN_HOEHE}."
        ));
    }

    terminal_vorbereiten()?;
    panic_hook_setzen();

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend).map_err(|e| e.to_string())?;

    let ergebnis = schleife(&mut terminal, lang);

    terminal_zuruecksetzen();
    ergebnis
}

fn terminal_vorbereiten() -> Result<(), String> {
    enable_raw_mode().map_err(|e| e.to_string())?;
    execute!(io::stdout(), EnterAlternateScreen).map_err(|e| e.to_string())
}

fn terminal_zuruecksetzen() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen);
}

/// Panic-Hook, der das Terminal aufräumt, bevor die Meldung erscheint.
fn panic_hook_setzen() {
    let vorheriger = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        terminal_zuruecksetzen();
        vorheriger(info);
    }));
}

fn schleife(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    lang: &str,
) -> Result<Exitcode, String> {
    let mut ui = Oberflaeche::neu(lang);
    let abbruch = CancelToken::new();
    let mut empfaenger: Option<mpsc::Receiver<Meldung>> = None;
    let mut exitcode = Exitcode::Erfolg;

    loop {
        terminal
            .draw(|frame| zeichnen(frame, &ui))
            .map_err(|e| e.to_string())?;

        // Nachrichten aus dem Arbeitsthread einsammeln.
        if let Some(kanal) = &empfaenger {
            let mut fertig = false;
            loop {
                match kanal.try_recv() {
                    Ok(Meldung::Fortschritt(p)) => ui.fortschritt = Some(*p),
                    Ok(Meldung::ScanFertig(bericht)) => {
                        ui.scan = Some(*bericht);
                        ui.ansicht = Ansicht::Auswahl;
                        ui.fortschritt = None;
                        fertig = true;
                    }
                    Ok(Meldung::CleanFertig(bericht)) => {
                        if !bericht.success && !bericht.cancelled {
                            exitcode = Exitcode::Fehlgeschlagen;
                        }
                        ui.clean = Some(*bericht);
                        ui.ansicht = Ansicht::Ergebnis;
                        ui.fortschritt = None;
                        fertig = true;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        fertig = true;
                        break;
                    }
                }
            }
            if fertig {
                empfaenger = None;
                abbruch.reset();
            }
        }

        // Eingaben abfragen, ohne zu blockieren – sonst friert die Anzeige
        // während eines Laufs ein.
        if event::poll(Duration::from_millis(80)).map_err(|e| e.to_string())? {
            if let Event::Key(KeyEvent {
                code,
                modifiers,
                kind: KeyEventKind::Press,
                ..
            }) = event::read().map_err(|e| e.to_string())?
            {
                match ui.taste(code, modifiers) {
                    Aktion::Beenden => break,
                    Aktion::Abbrechen => {
                        abbruch.cancel();
                        ui.hinweis = i18n::t(&ui.lang, "cli.aborted");
                    }
                    Aktion::Analysieren if empfaenger.is_none() => {
                        ui.ansicht = Ansicht::Laeuft { phase: "scan" };
                        ui.fortschritt = None;
                        empfaenger = Some(scan_starten(abbruch.clone()));
                    }
                    Aktion::Bereinigen if empfaenger.is_none() => {
                        if ui.auswahl.is_empty() {
                            ui.hinweis = i18n::t(&ui.lang, "cli.no_selection");
                        } else {
                            ui.ansicht = Ansicht::Laeuft { phase: "clean" };
                            ui.fortschritt = None;
                            empfaenger = Some(clean_starten(
                                ui.auswahl.clone(),
                                ui.trockenlauf,
                                abbruch.clone(),
                            ));
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(exitcode)
}

fn scan_starten(abbruch: CancelToken) -> mpsc::Receiver<Meldung> {
    let (sender, empfaenger) = mpsc::channel();
    let melder = sender.clone();

    std::thread::spawn(move || {
        let ctx = RunContext::new()
            .with_cancel(abbruch)
            .with_progress(Box::new(move |p| {
                let _ = melder.send(Meldung::Fortschritt(Box::new(p)));
            }));
        let bericht = engine::scan(&[], &ctx);
        let _ = sender.send(Meldung::ScanFertig(Box::new(bericht)));
    });

    empfaenger
}

fn clean_starten(
    ziele: Vec<String>,
    trockenlauf: bool,
    abbruch: CancelToken,
) -> mpsc::Receiver<Meldung> {
    let (sender, empfaenger) = mpsc::channel();
    let melder = sender.clone();

    std::thread::spawn(move || {
        let ctx = RunContext::new()
            .with_cancel(abbruch)
            .with_progress(Box::new(move |p| {
                let _ = melder.send(Meldung::Fortschritt(Box::new(p)));
            }));
        let anfrage = CleanRequest {
            targets: ziele,
            only_paths: Vec::new(),
            dry_run: trockenlauf,
        };
        let bericht = engine::clean(&anfrage, &super::run::sicherungsordner(), &ctx);
        let _ = sender.send(Meldung::CleanFertig(Box::new(bericht)));
    });

    empfaenger
}

// ---------------------------------------------------------------------------
// Zeichnen
// ---------------------------------------------------------------------------

fn zeichnen(frame: &mut Frame, ui: &Oberflaeche) {
    let flaeche = frame.area();
    if flaeche.width < MIN_BREITE || flaeche.height < MIN_HOEHE {
        frame.render_widget(
            Paragraph::new(format!(
                "Terminal zu klein – mindestens {MIN_BREITE}×{MIN_HOEHE} nötig."
            ))
            .alignment(Alignment::Center),
            flaeche,
        );
        return;
    }

    let bereiche = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(flaeche);

    kopfzeile(frame, bereiche[0], ui);

    match &ui.ansicht {
        Ansicht::Start => startbildschirm(frame, bereiche[1], ui),
        Ansicht::Auswahl => auswahlansicht(frame, bereiche[1], ui),
        Ansicht::Laeuft { phase } => laufansicht(frame, bereiche[1], ui, phase),
        Ansicht::Ergebnis => ergebnisansicht(frame, bereiche[1], ui),
        Ansicht::Hilfe => hilfeansicht(frame, bereiche[1], ui),
    }

    fusszeile(frame, bereiche[2], ui);
}

fn kopfzeile(frame: &mut Frame, flaeche: Rect, ui: &Oberflaeche) {
    let modus = if ui.trockenlauf {
        format!("  [{}]", i18n::t(&ui.lang, "home.dry_run"))
    } else {
        String::new()
    };

    let zeile = Line::from(vec![
        Span::styled(
            " PLANE ",
            Style::default().fg(AKZENT).add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("v{}", env!("CARGO_PKG_VERSION"))),
        Span::styled(modus, Style::default().fg(Color::Yellow)),
    ]);

    frame.render_widget(
        Paragraph::new(zeile).block(Block::default().borders(Borders::BOTTOM)),
        flaeche,
    );
}

fn startbildschirm(frame: &mut Frame, flaeche: Rect, ui: &Oberflaeche) {
    let mut zeilen: Vec<Line> = vec![Line::raw("")];
    for logo in LOGO {
        zeilen.push(Line::from(Span::styled(
            *logo,
            Style::default().fg(AKZENT).add_modifier(Modifier::BOLD),
        )));
    }
    zeilen.push(Line::raw(""));
    zeilen.push(Line::from(Span::styled(
        i18n::t(&ui.lang, "app.tagline"),
        Style::default().add_modifier(Modifier::DIM),
    )));
    zeilen.push(Line::raw(""));
    zeilen.push(Line::from(Span::styled(
        format!("  [s] {}", i18n::t(&ui.lang, "home.analyze")),
        Style::default().fg(AKZENT).add_modifier(Modifier::BOLD),
    )));

    frame.render_widget(Paragraph::new(zeilen).alignment(Alignment::Center), flaeche);
}

fn auswahlansicht(frame: &mut Frame, flaeche: Rect, ui: &Oberflaeche) {
    let spalten = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(26),
            Constraint::Min(30),
            Constraint::Length(34),
        ])
        .split(flaeche);

    kategoriespalte(frame, spalten[0], ui);
    zielspalte(frame, spalten[1], ui);
    detailspalte(frame, spalten[2], ui);
}

fn kategoriespalte(frame: &mut Frame, flaeche: Rect, ui: &Oberflaeche) {
    let eintraege: Vec<ListItem> = ui
        .kategorien
        .iter()
        .map(|kategorie| {
            let groesse = ui.kategorie_groesse(*kategorie);
            let name = text::kategoriename(&ui.lang, *kategorie);
            let wert = if groesse > 0 {
                format_bytes(groesse)
            } else {
                "–".to_string()
            };
            ListItem::new(Line::from(vec![
                Span::raw(format!("{name:<14}")),
                Span::styled(wert, Style::default().add_modifier(Modifier::DIM)),
            ]))
        })
        .collect();

    let rand = rahmen(
        i18n::t(&ui.lang, "nav.dashboard"),
        ui.spalte == Spalte::Kategorien,
    );
    let mut zustand = ListState::default();
    zustand.select(Some(ui.kategorie_index));

    frame.render_stateful_widget(
        List::new(eintraege)
            .block(rand)
            .highlight_style(Style::default().fg(AKZENT).add_modifier(Modifier::BOLD))
            .highlight_symbol("▍"),
        flaeche,
        &mut zustand,
    );
}

fn zielspalte(frame: &mut Frame, flaeche: Rect, ui: &Oberflaeche) {
    let eintraege: Vec<ListItem> = ui
        .ziele()
        .iter()
        .map(|ziel| {
            let kasten = if ui.ist_ausgewaehlt(ziel.key) {
                "[x]"
            } else {
                "[ ]"
            };
            let scan = ui.scan_fuer(ziel.key);
            let wert = match scan {
                Some(s) if s.skipped => i18n::t(&ui.lang, "status.skipped"),
                Some(s) if s.size > 0 => format_bytes(s.size),
                Some(s) if s.item_count > 0 => format!("{}×", s.item_count),
                _ => "–".to_string(),
            };
            let farbe = match ziel.risk {
                engine::Risk::Safe => Color::Reset,
                engine::Risk::Notice => Color::Yellow,
                engine::Risk::Caution => Color::Red,
            };

            ListItem::new(Line::from(vec![
                Span::raw(format!("{kasten} ")),
                Span::styled(text::risikosymbol(ziel.risk), Style::default().fg(farbe)),
                Span::raw(" "),
                Span::raw(text::kuerzen(&text::zielname(&ui.lang, ziel.key), 28)),
                Span::styled(
                    format!("  {wert}"),
                    Style::default().add_modifier(Modifier::DIM),
                ),
            ]))
        })
        .collect();

    let titel = format!(
        "{}  ({})",
        text::kategoriename(&ui.lang, ui.kategorie()),
        i18n::format(
            &ui.lang,
            "home.selected_size",
            &[&format_bytes(ui.auswahl_groesse())]
        )
    );

    let mut zustand = ListState::default();
    zustand.select(Some(ui.ziel_index));

    frame.render_stateful_widget(
        List::new(eintraege)
            .block(rahmen(titel, ui.spalte == Spalte::Ziele))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol(""),
        flaeche,
        &mut zustand,
    );
}

fn detailspalte(frame: &mut Frame, flaeche: Rect, ui: &Oberflaeche) {
    let mut zeilen: Vec<Line> = Vec::new();

    if let Some(ziel) = ui.aktuelles_ziel() {
        zeilen.push(Line::from(Span::styled(
            text::zielname(&ui.lang, ziel.key),
            Style::default().add_modifier(Modifier::BOLD),
        )));
        zeilen.push(Line::raw(""));
        zeilen.push(Line::raw(text::zielbeschreibung(&ui.lang, ziel.key)));
        zeilen.push(Line::raw(""));
        zeilen.push(Line::from(Span::styled(
            text::risikohinweis(&ui.lang, ziel.risk),
            Style::default().fg(match ziel.risk {
                engine::Risk::Safe => AKZENT,
                engine::Risk::Notice => Color::Yellow,
                engine::Risk::Caution => Color::Red,
            }),
        )));

        for merkmal in text::zielmerkmale(&ui.lang, ziel) {
            zeilen.push(Line::from(Span::styled(
                format!("• {merkmal}"),
                Style::default().add_modifier(Modifier::DIM),
            )));
        }

        if let Some(scan) = ui.scan_fuer(ziel.key) {
            zeilen.push(Line::raw(""));
            if scan.skipped {
                zeilen.push(Line::from(Span::styled(
                    text::meldung(&ui.lang, &scan.skip_reason),
                    Style::default().fg(Color::Yellow),
                )));
            } else {
                zeilen.push(Line::raw(i18n::format(
                    &ui.lang,
                    "home.found_total",
                    &[&format_bytes(scan.size), &scan.item_count.to_string()],
                )));
            }
            for warnung in text::meldungen(&ui.lang, &scan.warnings) {
                zeilen.push(Line::from(Span::styled(
                    format!("⚠ {warnung}"),
                    Style::default().fg(Color::Yellow),
                )));
            }
        }
    }

    frame.render_widget(
        Paragraph::new(zeilen)
            .block(rahmen(i18n::t(&ui.lang, "about.technical"), false))
            .wrap(Wrap { trim: true }),
        flaeche,
    );
}

fn laufansicht(frame: &mut Frame, flaeche: Rect, ui: &Oberflaeche, phase: &str) {
    let bereiche = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(flaeche);

    let titel = if phase == "scan" {
        i18n::t(&ui.lang, "home.analyzing")
    } else {
        i18n::t(&ui.lang, "home.cleaning")
    };
    frame.render_widget(
        Paragraph::new(titel).alignment(Alignment::Center),
        bereiche[0],
    );

    let (prozent, zielname, bytes, pfad, zaehler) = match &ui.fortschritt {
        Some(p) => (
            super::progress::begrenzen(p.percent),
            i18n::t(&ui.lang, &p.target_i18n),
            format_bytes(p.bytes),
            p.current_path.clone(),
            format!("{}/{}", p.index + 1, p.total),
        ),
        None => (
            0.0,
            String::new(),
            format_bytes(0),
            String::new(),
            String::new(),
        ),
    };

    frame.render_widget(
        Gauge::default()
            .block(Block::default().borders(Borders::ALL))
            .gauge_style(Style::default().fg(AKZENT))
            .ratio(prozent / 100.0)
            .label(format!("{prozent:.1} %  {zaehler}")),
        bereiche[1],
    );

    let mut zeilen = vec![
        Line::from(Span::styled(
            zielname,
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::raw(bytes),
    ];
    if !pfad.is_empty() {
        zeilen.push(Line::from(Span::styled(
            text::kuerzen(&pfad, flaeche.width.saturating_sub(4) as usize),
            Style::default().add_modifier(Modifier::DIM),
        )));
    }
    zeilen.push(Line::raw(""));
    zeilen.push(Line::from(Span::styled(
        format!("[Esc] {}", i18n::t(&ui.lang, "home.cancel")),
        Style::default().add_modifier(Modifier::DIM),
    )));

    frame.render_widget(
        Paragraph::new(zeilen).alignment(Alignment::Center),
        bereiche[2],
    );
}

fn ergebnisansicht(frame: &mut Frame, flaeche: Rect, ui: &Oberflaeche) {
    let Some(bericht) = &ui.clean else {
        return;
    };

    let mut zeilen = vec![
        Line::from(Span::styled(
            super::table::clean_zusammenfassung(&ui.lang, bericht, ui.trockenlauf),
            Style::default().fg(AKZENT).add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
    ];

    for ziel in &bericht.targets {
        let zeichen = text::statuszeichen(ziel.ok, ziel.skipped);
        let name = text::zielname(&ui.lang, &ziel.key);
        let wert = if ziel.skipped {
            text::meldung(&ui.lang, &ziel.skip_reason)
        } else {
            format_bytes(ziel.freed)
        };
        zeilen.push(Line::raw(format!("{zeichen} {name:<34} {wert}")));
        for fehler in text::meldungen(&ui.lang, &ziel.errors) {
            zeilen.push(Line::from(Span::styled(
                format!("    {fehler}"),
                Style::default().fg(Color::Red),
            )));
        }
    }

    if let Some(sicherung) = &bericht.registry_backup {
        zeilen.push(Line::raw(""));
        zeilen.push(Line::raw(i18n::format(
            &ui.lang,
            "clean.registry_backup",
            &[sicherung],
        )));
    }

    frame.render_widget(
        Paragraph::new(zeilen)
            .block(rahmen(i18n::t(&ui.lang, "status.ok"), false))
            .wrap(Wrap { trim: false }),
        flaeche,
    );
}

fn hilfeansicht(frame: &mut Frame, flaeche: Rect, ui: &Oberflaeche) {
    let tasten = [
        ("↑ ↓ / j k", "navigieren"),
        ("← → / h l", "Spalte wechseln"),
        ("Leertaste", "auswählen"),
        ("a / n / r", "alles / nichts / empfohlen"),
        ("s", "analysieren"),
        ("c", "bereinigen"),
        ("d", "Trockenlauf umschalten"),
        ("L", "Sprache umschalten"),
        ("? / F1", "diese Hilfe"),
        ("q / Esc", "beenden"),
    ];

    let zeilen: Vec<Line> = tasten
        .iter()
        .map(|(taste, zweck)| {
            Line::from(vec![
                Span::styled(
                    format!("  {taste:<12}"),
                    Style::default().fg(AKZENT).add_modifier(Modifier::BOLD),
                ),
                Span::raw(*zweck),
            ])
        })
        .collect();

    frame.render_widget(Clear, flaeche);
    frame.render_widget(
        Paragraph::new(zeilen).block(rahmen(i18n::t(&ui.lang, "nav.settings"), true)),
        flaeche,
    );
}

fn fusszeile(frame: &mut Frame, flaeche: Rect, ui: &Oberflaeche) {
    let hilfe = if matches!(ui.ansicht, Ansicht::Laeuft { .. }) {
        format!("[Esc] {}", i18n::t(&ui.lang, "home.cancel"))
    } else {
        format!(
            "[s] {}  [c] {}  [Leer] {}  [d] {}  [?] Hilfe  [q] Ende",
            i18n::t(&ui.lang, "home.analyze"),
            i18n::t(&ui.lang, "home.clean"),
            i18n::t(&ui.lang, "home.select_all"),
            i18n::t(&ui.lang, "home.dry_run"),
        )
    };

    let zeilen = vec![Line::from(Span::styled(
        if ui.hinweis.is_empty() {
            hilfe
        } else {
            format!("{}  —  {hilfe}", ui.hinweis)
        },
        Style::default().add_modifier(Modifier::DIM),
    ))];

    frame.render_widget(
        Paragraph::new(zeilen).block(Block::default().borders(Borders::TOP)),
        flaeche,
    );
}

fn rahmen(titel: String, aktiv: bool) -> Block<'static> {
    let stil = if aktiv {
        Style::default().fg(AKZENT)
    } else {
        Style::default()
    };
    Block::default()
        .borders(Borders::ALL)
        .border_style(stil)
        .title(titel)
}

/// Nur zur Vollständigkeit: wie lange die Oberfläche auf Eingaben wartet.
/// Bewusst kurz, damit der Fortschrittsbalken flüssig bleibt.
pub const POLL_INTERVALL: Duration = Duration::from_millis(80);

/// Zeitpunkt, an dem die Oberfläche gestartet wurde – für Tests der Laufzeit.
pub fn jetzt() -> Instant {
    Instant::now()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ui() -> Oberflaeche {
        Oberflaeche::neu("de")
    }

    #[test]
    fn startet_auf_dem_startbildschirm() {
        let o = ui();
        assert_eq!(o.ansicht, Ansicht::Start);
        assert_eq!(o.spalte, Spalte::Kategorien);
        assert!(!o.trockenlauf);
    }

    #[test]
    fn startauswahl_ist_die_empfohlene() {
        let o = ui();
        assert_eq!(o.auswahl.len(), engine::default_selection().len());
        for key in &o.auswahl {
            let ziel = engine::target_by_key(key).unwrap();
            assert_ne!(ziel.risk, engine::Risk::Caution, "{key} ist riskant");
        }
    }

    #[test]
    fn logo_ist_rechteckig() {
        let breite = LOGO[0].chars().count();
        for zeile in LOGO {
            assert_eq!(zeile.chars().count(), breite, "ungleiche Logozeile");
        }
        assert!(
            breite < MIN_BREITE as usize,
            "Logo passt nicht ins Terminal"
        );
    }

    #[test]
    fn navigation_bleibt_in_den_grenzen() {
        let mut o = ui();
        for _ in 0..50 {
            o.taste(KeyCode::Down, KeyModifiers::NONE);
        }
        assert_eq!(o.kategorie_index, o.kategorien.len() - 1);

        for _ in 0..50 {
            o.taste(KeyCode::Up, KeyModifiers::NONE);
        }
        assert_eq!(o.kategorie_index, 0);
    }

    #[test]
    fn spaltenwechsel_funktioniert() {
        let mut o = ui();
        o.taste(KeyCode::Right, KeyModifiers::NONE);
        assert_eq!(o.spalte, Spalte::Ziele);
        o.taste(KeyCode::Left, KeyModifiers::NONE);
        assert_eq!(o.spalte, Spalte::Kategorien);
    }

    #[test]
    fn kategoriewechsel_setzt_die_zielmarkierung_zurueck() {
        let mut o = ui();
        o.spalte = Spalte::Ziele;
        o.taste(KeyCode::Down, KeyModifiers::NONE);
        o.taste(KeyCode::Down, KeyModifiers::NONE);
        assert!(o.ziel_index > 0);

        o.spalte = Spalte::Kategorien;
        o.taste(KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(o.ziel_index, 0);
    }

    #[test]
    fn leertaste_schaltet_ein_ziel_um() {
        let mut o = ui();
        o.spalte = Spalte::Ziele;
        let key = o.aktuelles_ziel().unwrap().key.to_string();
        let vorher = o.ist_ausgewaehlt(&key);

        o.taste(KeyCode::Char(' '), KeyModifiers::NONE);
        assert_ne!(o.ist_ausgewaehlt(&key), vorher);

        o.taste(KeyCode::Char(' '), KeyModifiers::NONE);
        assert_eq!(o.ist_ausgewaehlt(&key), vorher);
    }

    #[test]
    fn leertaste_in_der_kategoriespalte_schaltet_die_ganze_kategorie() {
        let mut o = ui();
        o.nichts_waehlen();
        let kategorie = o.kategorie();
        let keys: Vec<String> = engine::targets_in(kategorie)
            .map(|t| t.key.to_string())
            .collect();

        o.taste(KeyCode::Char(' '), KeyModifiers::NONE);
        assert!(keys.iter().all(|k| o.ist_ausgewaehlt(k)));

        o.taste(KeyCode::Char(' '), KeyModifiers::NONE);
        assert!(keys.iter().all(|k| !o.ist_ausgewaehlt(k)));
    }

    #[test]
    fn alles_nichts_empfohlen() {
        let mut o = ui();

        o.taste(KeyCode::Char('a'), KeyModifiers::NONE);
        assert_eq!(o.auswahl.len(), engine::TARGETS.len());

        o.taste(KeyCode::Char('n'), KeyModifiers::NONE);
        assert!(o.auswahl.is_empty());

        o.taste(KeyCode::Char('r'), KeyModifiers::NONE);
        assert_eq!(o.auswahl.len(), engine::default_selection().len());
    }

    #[test]
    fn trockenlauf_laesst_sich_umschalten() {
        let mut o = ui();
        o.taste(KeyCode::Char('d'), KeyModifiers::NONE);
        assert!(o.trockenlauf);
        o.taste(KeyCode::Char('d'), KeyModifiers::NONE);
        assert!(!o.trockenlauf);
    }

    #[test]
    fn sprache_laesst_sich_umschalten() {
        let mut o = ui();
        assert_eq!(o.lang, "de");
        o.taste(KeyCode::Char('L'), KeyModifiers::NONE);
        assert_eq!(o.lang, "en");
        o.taste(KeyCode::Char('L'), KeyModifiers::NONE);
        assert_eq!(o.lang, "de");
    }

    #[test]
    fn s_loest_die_analyse_aus() {
        let mut o = ui();
        assert_eq!(
            o.taste(KeyCode::Char('s'), KeyModifiers::NONE),
            Aktion::Analysieren
        );
    }

    #[test]
    fn c_loest_die_bereinigung_aus() {
        let mut o = ui();
        assert_eq!(
            o.taste(KeyCode::Char('c'), KeyModifiers::NONE),
            Aktion::Bereinigen
        );
    }

    #[test]
    fn q_beendet() {
        let mut o = ui();
        assert_eq!(
            o.taste(KeyCode::Char('q'), KeyModifiers::NONE),
            Aktion::Beenden
        );
        assert_eq!(o.taste(KeyCode::Esc, KeyModifiers::NONE), Aktion::Beenden);
    }

    #[test]
    fn waehrend_eines_laufs_ist_nur_abbruch_moeglich() {
        let mut o = ui();
        o.ansicht = Ansicht::Laeuft { phase: "scan" };

        assert_eq!(
            o.taste(KeyCode::Char('a'), KeyModifiers::NONE),
            Aktion::Nichts
        );
        assert!(
            o.auswahl.len() < engine::TARGETS.len(),
            "Auswahl darf sich während eines Laufs nicht ändern"
        );
        assert_eq!(o.taste(KeyCode::Esc, KeyModifiers::NONE), Aktion::Abbrechen);
    }

    #[test]
    fn strg_c_beendet_beziehungsweise_bricht_ab() {
        let mut o = ui();
        assert_eq!(
            o.taste(KeyCode::Char('c'), KeyModifiers::CONTROL),
            Aktion::Beenden
        );

        o.ansicht = Ansicht::Laeuft { phase: "clean" };
        assert_eq!(
            o.taste(KeyCode::Char('c'), KeyModifiers::CONTROL),
            Aktion::Abbrechen
        );
    }

    #[test]
    fn hilfe_oeffnet_und_schliesst_sich() {
        let mut o = ui();
        o.taste(KeyCode::Char('?'), KeyModifiers::NONE);
        assert_eq!(o.ansicht, Ansicht::Hilfe);

        o.taste(KeyCode::Char('x'), KeyModifiers::NONE);
        assert_eq!(o.ansicht, Ansicht::Start);
    }

    #[test]
    fn hilfe_kehrt_nach_der_analyse_zur_auswahl_zurueck() {
        let mut o = ui();
        o.scan = Some(engine::scan(
            &["system.dns".to_string()],
            &RunContext::new().with_elevated(false),
        ));
        o.ansicht = Ansicht::Hilfe;
        o.taste(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(o.ansicht, Ansicht::Auswahl);
    }

    #[test]
    fn auswahlgroesse_zaehlt_nur_gewaehlte_ziele() {
        let mut o = ui();
        assert_eq!(o.auswahl_groesse(), 0, "ohne Analyse keine Größe");

        o.scan = Some(engine::scan(
            &["system.dns".to_string()],
            &RunContext::new().with_elevated(false),
        ));
        o.nichts_waehlen();
        assert_eq!(o.auswahl_groesse(), 0);
    }

    #[test]
    fn jede_kategorie_hat_ziele() {
        let mut o = ui();
        for index in 0..o.kategorien.len() {
            o.kategorie_index = index;
            assert!(!o.ziele().is_empty(), "{:?} ist leer", o.kategorie());
        }
    }

    #[test]
    fn aktuelles_ziel_ist_immer_gueltig() {
        let mut o = ui();
        o.ziel_index = 9999;
        assert!(o.aktuelles_ziel().is_some(), "Index muss begrenzt werden");
    }
}
