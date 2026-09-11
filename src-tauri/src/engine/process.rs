//! Ausführung externer Befehle – ohne Shell, mit Zeitlimit.
//!
//! Einziger Ort im Projekt, an dem Prozesse gestartet werden. Argumente werden
//! als Liste übergeben; es gibt damit keine Command-Injection-Fläche und keine
//! Abhängigkeit von cmd.exe- oder PowerShell-Expansionsregeln.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Zeitlimit für externe Befehle.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);

/// Ergebnis eines Prozessaufrufs.
#[derive(Debug, Clone)]
pub struct Output {
    /// Exitcode; `-1` bei Zeitüberschreitung oder Startfehler.
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Output {
    pub fn ok(&self) -> bool {
        self.code == 0
    }

    /// Aussagekräftigste Fehlermeldung.
    pub fn message(&self) -> String {
        if !self.stderr.is_empty() {
            self.stderr.clone()
        } else if !self.stdout.is_empty() {
            self.stdout.clone()
        } else {
            format!("Exitcode {}", self.code)
        }
    }

    fn fehler(text: impl Into<String>) -> Self {
        Self {
            code: -1,
            stdout: String::new(),
            stderr: text.into(),
        }
    }
}

/// Befehl mit Standardzeitlimit ausführen.
pub fn run(argv: &[&str]) -> Output {
    run_with_timeout(argv, DEFAULT_TIMEOUT)
}

/// Befehl mit eigenem Zeitlimit ausführen.
///
/// `std::process` kennt kein Zeitlimit; der Prozess wird deshalb beobachtet und
/// nach Ablauf beendet.
pub fn run_with_timeout(argv: &[&str], timeout: Duration) -> Output {
    let Some((programm, argumente)) = argv.split_first() else {
        return Output::fehler("Leerer Befehl");
    };

    let mut befehl = Command::new(programm);
    befehl
        .args(argumente)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW – verhindert aufblitzende Konsolenfenster.
        befehl.creation_flags(0x0800_0000);
    }

    let mut kind = match befehl.spawn() {
        Ok(k) => k,
        Err(e) => return Output::fehler(e.to_string()),
    };

    let start = Instant::now();
    loop {
        match kind.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = kind.kill();
                    let _ = kind.wait();
                    return Output::fehler(format!(
                        "Zeitlimit von {}s überschritten",
                        timeout.as_secs()
                    ));
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(e) => return Output::fehler(e.to_string()),
        }
    }

    match kind.wait_with_output() {
        Ok(ausgabe) => Output {
            code: ausgabe.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&ausgabe.stdout).trim().to_string(),
            stderr: String::from_utf8_lossy(&ausgabe.stderr).trim().to_string(),
        },
        Err(e) => Output::fehler(e.to_string()),
    }
}

/// Dienste stoppen oder starten.
///
/// `aktion` ist `"Stop"` oder `"Start"`.
pub fn service(aktion: &str, dienste: &[&str]) -> Output {
    if dienste.is_empty() {
        return Output {
            code: 0,
            stdout: String::new(),
            stderr: String::new(),
        };
    }
    let skript = format!(
        "{aktion}-Service -Name {} -Force -ErrorAction Stop",
        dienste.join(",")
    );
    run(&[
        "powershell",
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        &skript,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leerer_befehl_wird_abgefangen() {
        let ausgabe = run(&[]);
        assert_eq!(ausgabe.code, -1);
        assert_eq!(ausgabe.stderr, "Leerer Befehl");
        assert!(!ausgabe.ok());
    }

    #[test]
    fn unbekanntes_programm_liefert_fehler_statt_panik() {
        let ausgabe = run(&["plane_gibt_es_nicht_98765"]);
        assert_eq!(ausgabe.code, -1);
        assert!(!ausgabe.message().is_empty());
    }

    #[test]
    fn erfolgreicher_befehl_liefert_ausgabe() {
        let ausgabe = if cfg!(windows) {
            run(&["cmd", "/c", "echo", "hallo"])
        } else {
            run(&["echo", "hallo"])
        };
        assert!(ausgabe.ok(), "{}", ausgabe.message());
        assert!(ausgabe.stdout.contains("hallo"));
    }

    #[test]
    fn zeitlimit_beendet_haengende_prozesse() {
        let ausgabe = if cfg!(windows) {
            run_with_timeout(
                &["cmd", "/c", "ping", "-n", "30", "127.0.0.1"],
                Duration::from_millis(300),
            )
        } else {
            run_with_timeout(&["sleep", "30"], Duration::from_millis(300))
        };
        assert_eq!(ausgabe.code, -1);
        assert!(ausgabe.stderr.contains("Zeitlimit"));
    }

    #[test]
    fn leere_dienstliste_ist_erfolgreich() {
        assert!(service("Stop", &[]).ok());
    }

    #[test]
    fn message_bevorzugt_stderr() {
        let ausgabe = Output {
            code: 1,
            stdout: "aus".into(),
            stderr: "fehler".into(),
        };
        assert_eq!(ausgabe.message(), "fehler");

        let nur_stdout = Output {
            code: 1,
            stdout: "aus".into(),
            stderr: String::new(),
        };
        assert_eq!(nur_stdout.message(), "aus");

        let stumm = Output {
            code: 3,
            stdout: String::new(),
            stderr: String::new(),
        };
        assert_eq!(stumm.message(), "Exitcode 3");
    }
}
