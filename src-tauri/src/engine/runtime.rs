//! Laufzeitkontext eines Durchlaufs: Abbruch, Fortschritt, Rechte, Prozesse.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use super::types::Progress;

/// Abbruchsignal. Wird von der Oberfläche gesetzt und von Scan/Clean zwischen
/// den Einzelschritten geprüft.
#[derive(Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn reset(&self) {
        self.0.store(false, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// Empfänger für Fortschrittsmeldungen.
///
/// Die Engine kennt Tauri nicht – sie ruft nur diesen Callback auf. Dadurch
/// lässt sie sich in Tests und in der CLI genauso verwenden wie in der App.
pub type ProgressSink = Box<dyn Fn(Progress) + Send + Sync>;

/// Kontext eines Durchlaufs.
pub struct RunContext {
    pub cancel: CancelToken,
    pub progress: Option<ProgressSink>,
    /// Erhöhte Rechte vorhanden.
    pub elevated: bool,
}

impl Default for RunContext {
    fn default() -> Self {
        Self {
            cancel: CancelToken::new(),
            progress: None,
            elevated: is_admin(),
        }
    }
}

impl RunContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_progress(mut self, sink: ProgressSink) -> Self {
        self.progress = Some(sink);
        self
    }

    pub fn with_cancel(mut self, token: CancelToken) -> Self {
        self.cancel = token;
        self
    }

    /// Nur für Tests: Rechtestatus vortäuschen.
    pub fn with_elevated(mut self, elevated: bool) -> Self {
        self.elevated = elevated;
        self
    }

    pub fn report(&self, fortschritt: Progress) {
        if let Some(sink) = &self.progress {
            sink(fortschritt);
        }
    }

    pub fn cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }
}

/// `true`, wenn der Prozess mit erhöhten Rechten läuft.
#[cfg(windows)]
pub fn is_admin() -> bool {
    // SAFETY: IsUserAnAdmin nimmt keine Argumente und hat keine Speicherwirkung.
    unsafe { windows_sys::Win32::UI::Shell::IsUserAnAdmin() != 0 }
}

#[cfg(not(windows))]
pub fn is_admin() -> bool {
    false
}

/// Laufende Prozesse aus einer Liste ermitteln (Dateinamen, ohne Pfad).
///
/// Wird genutzt, um zu warnen, dass ein Browser-Cache nur unvollständig
/// gelöscht werden kann, solange der Browser läuft.
pub fn running_processes(gesucht: &[&str]) -> Vec<String> {
    if gesucht.is_empty() {
        return Vec::new();
    }

    use sysinfo::{ProcessRefreshKind, RefreshKind, System};

    let system = System::new_with_specifics(
        RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing()),
    );

    let mut gefunden: Vec<String> = Vec::new();
    for prozess in system.processes().values() {
        let name = prozess.name().to_string_lossy().to_string();
        if gesucht.iter().any(|g| g.eq_ignore_ascii_case(&name)) && !gefunden.contains(&name) {
            gefunden.push(name);
        }
    }
    gefunden
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn abbruchsignal_wirkt_und_laesst_sich_zuruecksetzen() {
        let token = CancelToken::new();
        assert!(!token.is_cancelled());
        token.cancel();
        assert!(token.is_cancelled());
        token.reset();
        assert!(!token.is_cancelled());
    }

    #[test]
    fn abbruchsignal_wird_geteilt() {
        let token = CancelToken::new();
        let klon = token.clone();
        klon.cancel();
        assert!(token.is_cancelled(), "Klon teilt den Zustand nicht");
    }

    #[test]
    fn kontext_ohne_sink_meldet_ohne_absturz() {
        let kontext = RunContext::new();
        kontext.report(Progress {
            phase: "scan",
            target_key: "x".into(),
            target_i18n: "target.x.name".into(),
            index: 0,
            total: 1,
            percent: 0.0,
            bytes: 0,
            current_path: String::new(),
            done: false,
        });
    }

    #[test]
    fn kontext_reicht_fortschritt_an_den_sink_weiter() {
        let zaehler = Arc::new(AtomicUsize::new(0));
        let kopie = Arc::clone(&zaehler);
        let kontext = RunContext::new().with_progress(Box::new(move |_| {
            kopie.fetch_add(1, Ordering::SeqCst);
        }));

        for _ in 0..3 {
            kontext.report(Progress {
                phase: "clean",
                target_key: "x".into(),
                target_i18n: "target.x.name".into(),
                index: 0,
                total: 1,
                percent: 0.0,
                bytes: 0,
                current_path: String::new(),
                done: false,
            });
        }
        assert_eq!(zaehler.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn leere_prozessliste_fragt_das_system_nicht_ab() {
        assert!(running_processes(&[]).is_empty());
    }

    #[test]
    fn running_processes_findet_den_eigenen_prozess() {
        let eigener = std::env::current_exe()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()));
        if let Some(name) = eigener {
            let gefunden = running_processes(&[&name]);
            assert!(
                !gefunden.is_empty(),
                "Der eigene Prozess {name} wurde nicht gefunden"
            );
        }
    }
}
