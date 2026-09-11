//! Einstiegspunkt der Kommandozeile und Textoberfläche von Plane.
//!
//! Die Logik liegt in `plane_lib::cli`; hier wird nur ausgewertet, ein
//! Abbruchsignal für Strg+C installiert und der Exitcode gesetzt.
//!
//! ```text
//! plane-cli list                  alle Reinigungsziele auflisten
//! plane-cli scan                  analysieren, ohne etwas zu verändern
//! plane-cli clean --dry-run       simulieren
//! plane-cli clean -y              empfohlene Auswahl bereinigen
//! plane-cli info                  System und Rechtestatus
//! plane-cli tui                   Textoberfläche (Standard im Terminal)
//! ```

use clap::Parser;

use plane_lib::cli::{ausfuehren, Cli};

fn main() {
    let cli = Cli::parse();
    let exitcode = ausfuehren(cli);
    std::process::exit(exitcode.wert());
}
