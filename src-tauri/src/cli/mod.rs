//! Kommandozeile und Textoberfläche.
//!
//! `plane-cli` nutzt dieselbe Engine wie die grafische Anwendung – es gibt
//! keine zweite Reinigungslogik. Der Aufbau trennt konsequent, was sich prüfen
//! lässt, von dem, was auf das Terminal schreibt:
//!
//! ```text
//! args      Argumente auswerten, Auswahl prüfen, Exitcodes ableiten
//! text      Übersetzungsschlüssel auflösen, Namen und Symbole
//! table     Tabellen und Zusammenfassungen aufbereiten
//! progress  Fortschrittsbalken als Text
//! info      Systeminformationen einsammeln
//! stil      Farbcodes, abschaltbar
//! run       Unterbefehle ausführen – der einzige Ort mit println!
//! tui       Textoberfläche (ratatui)
//! ```
//!
//! Alles außer `run` und `tui` ist ohne Terminal testbar; genau dort liegen
//! deshalb die Unit-Tests.

pub mod args;
pub mod info;
pub mod progress;
pub mod run;
pub mod stil;
pub mod table;
pub mod text;
pub mod tui;

pub use args::{Cli, Exitcode};
pub use run::ausfuehren;
