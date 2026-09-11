//! Sprachkatalog – eine Quelle der Wahrheit für GUI, CLI und TUI.
//!
//! Die Übersetzungen liegen bewusst **hier** und nicht als JSON im Frontend:
//! Zielnamen werden von der Oberfläche *und* von der Kommandozeile gebraucht.
//! Das Frontend holt sich den passenden Satz über den Command `get_translations`.
//!
//! # Neue Texte hinzufügen
//!
//! Einen Eintrag in [`ENTRIES`] ergänzen – Schlüssel, deutscher Text,
//! englischer Text. Der Test `jeder_katalogeintrag_ist_uebersetzt` stellt
//! sicher, dass kein Reinigungsziel ohne Übersetzung bleibt.
//!
//! # Platzhalter
//!
//! Texte können `{0}`, `{1}`, … enthalten. [`format`] ersetzt sie der Reihe
//! nach; die Oberfläche nutzt dieselbe Konvention.

use std::collections::BTreeMap;

/// Unterstützte Sprachen: Kürzel und Eigenbezeichnung.
pub const LANGUAGES: &[(&str, &str)] = &[("de", "Deutsch"), ("en", "English")];

/// Sprache, die verwendet wird, wenn keine passt.
pub const FALLBACK: &str = "en";

/// Alle Texte: `(Schlüssel, Deutsch, English)`.
#[rustfmt::skip]
pub const ENTRIES: &[(&str, &str, &str)] = &[
    // --- Anwendung -------------------------------------------------------
    ("app.tagline", "Der schlanke PC-Cleaner – lokal, schnell, ohne Datenübertragung.", "The lightweight PC cleaner – local, fast, no data leaves your device."),
    ("app.welcome_title", "Willkommen bei Plane", "Welcome to Plane"),
    ("app.start", "Loslegen", "Get started"),

    // --- Navigation ------------------------------------------------------
    ("nav.dashboard", "Übersicht", "Dashboard"),
    ("nav.about", "Über Plane", "About Plane"),
    ("nav.settings", "Einstellungen", "Settings"),
    ("nav.back", "Zurück zur Übersicht", "Back to dashboard"),

    // --- Übersicht -------------------------------------------------------
    ("home.title", "Aufräumen", "Clean up"),
    ("home.subtitle", "Plane prüft erst, was sich lohnt – gelöscht wird nur, was Sie auswählen.", "Plane checks what is worth cleaning first – only what you select gets deleted."),
    ("home.analyze", "Analysieren", "Analyze"),
    ("home.analyzing", "Analysiere …", "Analyzing …"),
    ("home.clean", "Bereinigen", "Clean"),
    ("home.cleaning", "Bereinige …", "Cleaning …"),
    ("home.cancel", "Abbrechen", "Cancel"),
    ("home.select_all", "Alles auswählen", "Select all"),
    ("home.select_none", "Auswahl aufheben", "Clear selection"),
    ("home.select_recommended", "Empfohlene Auswahl", "Recommended selection"),
    ("home.dry_run", "Nur simulieren (nichts löschen)", "Simulate only (delete nothing)"),
    ("home.never_scanned", "Noch nicht analysiert", "Not analyzed yet"),
    ("home.nothing_found", "Nichts zu bereinigen – alles sauber.", "Nothing to clean – all tidy."),
    ("home.selected_size", "{0} ausgewählt", "{0} selected"),
    ("home.found_total", "{0} in {1} Einträgen gefunden", "{0} found across {1} items"),

    // --- Laufwerk --------------------------------------------------------
    ("disk.title", "Systemlaufwerk {0}", "System drive {0}"),
    ("disk.free", "{0} frei", "{0} free"),
    ("disk.of_total", "von {0}", "of {0}"),
    ("disk.used_percent", "{0} % belegt", "{0}% used"),

    // --- Kategorien ------------------------------------------------------
    ("category.system", "System", "System"),
    ("category.browsers", "Browser", "Browsers"),
    ("category.apps", "Anwendungen", "Applications"),
    ("category.installers", "Installationsdateien", "Installer files"),
    ("category.recyclebin", "Papierkorb", "Recycle Bin"),
    ("category.registry", "Registry", "Registry"),

    // --- Risikostufen ----------------------------------------------------
    ("risk.safe", "Unbedenklich", "Safe"),
    ("risk.notice", "Mit Nebenwirkung", "Has side effects"),
    ("risk.caution", "Vorsicht", "Caution"),
    ("risk.safe.hint", "Wird bei Bedarf automatisch neu erzeugt.", "Recreated automatically when needed."),
    ("risk.notice.hint", "Spürbare Nebenwirkung – bitte Beschreibung lesen.", "Noticeable side effect – please read the description."),
    ("risk.caution.hint", "Kann Daten unwiderruflich entfernen.", "May remove data irreversibly."),
    ("risk.needs_admin", "Administratorrechte nötig", "Requires administrator rights"),
    ("risk.suggestion_only", "Nur Vorschlag – Einzelauswahl nötig", "Suggestion only – select individual files"),

    // --- Zustände und Meldungen -----------------------------------------
    ("status.skipped", "Übersprungen", "Skipped"),
    ("status.failed", "Fehlgeschlagen", "Failed"),
    ("status.ok", "Erledigt", "Done"),
    ("status.cancelled", "Abgebrochen", "Cancelled"),
    ("skip.needs_admin", "Übersprungen: Plane läuft ohne Administratorrechte", "Skipped: Plane is running without administrator rights"),
    ("skip.needs_explicit_selection", "Übersprungen: Bitte einzelne Dateien auswählen", "Skipped: please select individual files"),
    ("warn.process_running", "{0} läuft – gesperrte Dateien bleiben erhalten", "{0} is running – locked files will remain"),
    ("error.service_stop", "Dienst konnte nicht gestoppt werden: {0}", "Could not stop service: {0}"),
    ("error.service_start", "Dienst konnte nicht neu gestartet werden: {0}", "Could not restart service: {0}"),
    ("error.registry_backup", "Registry-Sicherung fehlgeschlagen – es wurde nichts geändert: {0}", "Registry backup failed – nothing was changed: {0}"),
    ("clean.failed", "Fehlgeschlagen: {0}", "Failed: {0}"),
    ("clean.summary", "{0} freigegeben, {1} Einträge entfernt", "{0} freed, {1} items removed"),
    ("clean.dry_run_summary", "Simulation: {0} würden freigegeben ({1} Einträge)", "Simulation: {0} would be freed ({1} items)"),
    ("clean.registry_backup", "Registry gesichert unter {0}", "Registry backed up to {0}"),
    ("clean.locked", "{0} Einträge waren in Benutzung und bleiben erhalten", "{0} items were in use and were kept"),
    ("clean.locked_hint", "Das ist normal – ein laufendes Programm hält seine Dateien offen. Administratorrechte ändern daran nichts; schließen Sie das Programm und starten Sie erneut.", "This is normal – a running program keeps its files open. Administrator rights do not help; close the program and run again."),
    ("clean.denied", "{0} Einträge brauchten höhere Rechte", "{0} items needed higher privileges"),
    ("clean.denied_hint", "Mit Administratorrechten neu starten, um diese Einträge zu entfernen.", "Restart with administrator rights to remove these items."),
    ("clean.log_written", "Protokoll: {0}", "Log: {0}"),

    // --- Administratorrechte ---------------------------------------------
    ("admin.restart", "Als Administrator neu starten", "Restart as administrator"),
    ("admin.restart_hint", "Windows fragt dabei nach Ihrer Zustimmung. Plane startet neu; nicht gespeicherte Auswahl geht verloren.", "Windows will ask for your consent. Plane restarts; an unsaved selection is lost."),
    ("admin.already", "Plane läuft bereits mit Administratorrechten.", "Plane is already running with administrator rights."),
    ("admin.failed", "Neustart als Administrator abgebrochen oder fehlgeschlagen.", "Restart as administrator was cancelled or failed."),

    // --- Deinstallation ---------------------------------------------------
    ("uninstall.title", "Programme", "Programs"),
    ("uninstall.subtitle", "Installierte Programme ansehen und entfernen. Plane startet den Deinstaller des jeweiligen Herstellers – es löscht nichts selbst.", "View and remove installed programs. Plane starts each vendor's own uninstaller – it deletes nothing itself."),
    ("uninstall.search", "Suchen", "Search"),
    ("uninstall.count", "{0} Programme, {1} gesperrt", "{0} programs, {1} protected"),
    ("uninstall.remove", "Deinstallieren", "Uninstall"),
    ("uninstall.removing", "Deinstalliere {0} …", "Uninstalling {0} …"),
    ("uninstall.confirm", "„{0}“ wirklich deinstallieren? Plane startet dazu den Deinstaller des Herstellers.", "Really uninstall “{0}”? Plane will start the vendor’s uninstaller."),
    ("uninstall.quiet_hint", "Läuft ohne weitere Rückfrage durch.", "Runs through without further prompts."),
    ("uninstall.loud_hint", "Der Deinstaller öffnet ein eigenes Fenster.", "The uninstaller opens its own window."),
    ("uninstall.needs_admin", "Braucht Administratorrechte", "Requires administrator rights"),
    ("uninstall.done", "„{0}“ wurde entfernt.", "“{0}” was removed."),
    ("uninstall.unverified", "Der Deinstaller meldet Erfolg, der Eintrag ist aber noch vorhanden. Manche Programme räumen verzögert auf – bitte später prüfen.", "The uninstaller reported success but the entry is still there. Some programs clean up with a delay – please check again later."),
    ("uninstall.not_installed", "War bereits nicht mehr installiert.", "Was already not installed."),
    ("uninstall.user_cancelled", "Vom Nutzer abgebrochen.", "Cancelled by the user."),
    ("uninstall.installer_busy", "Eine andere Installation läuft gerade. Bitte später erneut versuchen.", "Another installation is running. Please try again later."),
    ("uninstall.blocked_by_policy", "Durch eine Richtlinie Ihrer Organisation verboten.", "Blocked by a policy of your organization."),
    ("uninstall.failed", "Die Deinstallation ist fehlgeschlagen.", "The uninstallation failed."),
    ("uninstall.reboot_required", "Erfolgreich – ein Neustart schließt die Entfernung ab.", "Succeeded – a restart completes the removal."),
    ("uninstall.reboot_started", "Erfolgreich – Windows startet den Rechner neu.", "Succeeded – Windows is restarting the computer."),
    ("uninstall.no_command", "Für diesen Eintrag ist kein Deinstaller hinterlegt.", "No uninstaller is registered for this entry."),
    ("uninstall.windows_only", "Deinstallation gibt es nur unter Windows.", "Uninstalling is only available on Windows."),
    ("uninstall.protected", "Dieser Eintrag ist geschützt und wird nicht entfernt.", "This entry is protected and will not be removed."),
    ("uninstall.protected.runtime", "Laufzeitpaket – andere Programme brauchen es", "Runtime package – other programs depend on it"),
    ("uninstall.protected.security", "Sicherheitssoftware", "Security software"),
    ("uninstall.protected.driver", "Treiberpaket", "Driver package"),
    ("uninstall.protected.update", "Windows-Update", "Windows update"),
    ("uninstall.protected.system", "Systembestandteil von Windows", "Part of Windows itself"),
    ("uninstall.protected.self", "Das ist Plane selbst", "This is Plane itself"),
    ("uninstall.protected.noremove", "Windows bietet für diesen Eintrag keine Deinstallation an", "Windows does not offer to uninstall this entry"),
    ("uninstall.protected.no_uninstaller", "Kein Deinstaller hinterlegt", "No uninstaller registered"),
    ("uninstall.source.machine", "Für alle Benutzer", "All users"),
    ("uninstall.source.machine32", "Für alle Benutzer (32-Bit)", "All users (32-bit)"),
    ("uninstall.source.user", "Nur für Sie", "Just for you"),
    ("uninstall.source.store", "Store-App", "Store app"),

    // --- Protokoll --------------------------------------------------------
    ("log.title", "Protokoll", "Log"),
    ("log.open", "Protokoll öffnen", "Open log"),
    ("log.hint", "Jeder Lauf wird mitgeschrieben – hilfreich, wenn etwas nicht wie erwartet lief.", "Every run is recorded – useful when something did not go as expected."),
    ("log.none", "Noch kein Protokoll vorhanden.", "No log yet."),

    // --- Bestätigung -----------------------------------------------------
    ("confirm.title", "Bestätigung erforderlich", "Confirmation required"),
    ("confirm.intro", "Die folgenden Punkte werden ohne Freigabe übersprungen:", "The following items are skipped without your approval:"),
    ("confirm.warning", "Unwiderruflich gelöschte Daten lassen sich nicht wiederherstellen.", "Data deleted irreversibly cannot be restored."),
    ("confirm.accept", "Trotzdem ausführen", "Run anyway"),
    ("confirm.cancel", "Abbrechen", "Cancel"),

    // --- Einstellungen ---------------------------------------------------
    ("settings.title", "Einstellungen", "Settings"),
    ("settings.language", "Sprache", "Language"),
    ("settings.language.hint", "Gilt sofort für die gesamte Oberfläche.", "Applies immediately across the interface."),
    ("settings.theme", "Erscheinungsbild", "Appearance"),
    ("settings.theme.system", "Wie das System", "Match system"),
    ("settings.theme.light", "Hell", "Light"),
    ("settings.theme.dark", "Dunkel", "Dark"),
    ("settings.confirm_risky", "Vor riskanten Schritten nachfragen", "Ask before risky steps"),
    ("settings.confirm_risky.hint", "Dringend empfohlen. Ohne Nachfrage löscht Plane auch Unwiderrufliches.", "Strongly recommended. Without it Plane also deletes irreversible items."),
    ("settings.dry_run_default", "Standardmäßig nur simulieren", "Simulate by default"),
    ("settings.dry_run_default.hint", "Zeigt bei jedem Lauf nur an, was passieren würde.", "Every run only shows what would happen."),
    ("settings.reset_welcome", "Willkommensbildschirm erneut zeigen", "Show welcome screen again"),
    ("settings.close", "Schließen", "Close"),
    ("settings.saved", "Gespeichert", "Saved"),

    // --- Über ------------------------------------------------------------
    ("about.subtitle", "PC-Cleaner für Windows", "PC cleaner for Windows"),
    ("about.made_in", "Entwickelt mit Windows und Liebe in Deutschland.", "Made with Windows and love in Germany."),
    ("about.technical", "Technische Details", "Technical details"),
    ("about.opensource", "Open Source", "Open source"),
    ("about.system", "System", "System"),
    ("about.admin_yes", "Mit Administratorrechten gestartet", "Running with administrator rights"),
    ("about.admin_no", "Ohne Administratorrechte – einige Ziele werden übersprungen", "Without administrator rights – some targets are skipped"),

    // --- Ziele: System ---------------------------------------------------
    ("target.system.temp.user.name", "Temporäre Dateien", "Temporary files"),
    ("target.system.temp.user.description", "Der Ablageordner, in dem Programme Zwischendateien anlegen und oft vergessen.", "The folder where programs drop scratch files and often forget them."),
    ("target.system.temp.windows.name", "Temporäre Systemdateien", "System temporary files"),
    ("target.system.temp.windows.description", "Zwischendateien, die Windows selbst und Installationsprogramme hinterlassen.", "Scratch files left behind by Windows itself and by installers."),
    ("target.system.thumbnails.name", "Miniaturansichten", "Thumbnail cache"),
    ("target.system.thumbnails.description", "Vorschaubilder und Symbolzwischenspeicher des Explorers. Der erste Ordneraufruf dauert danach kurz länger.", "Explorer's preview images and icon cache. Opening a folder is slightly slower the first time afterwards."),
    ("target.system.inetcache.name", "Internet-Zwischenspeicher", "Internet cache"),
    ("target.system.inetcache.description", "Der systemweite Zwischenspeicher, den Windows-Komponenten für Webinhalte nutzen.", "The system-wide cache Windows components use for web content."),
    ("target.system.errorreports.name", "Fehlerberichte", "Error reports"),
    ("target.system.errorreports.description", "Berichte über abgestürzte Programme, die nie gesendet wurden.", "Reports about crashed programs that were never sent."),
    ("target.system.crashdumps.name", "Absturzabbilder", "Crash dumps"),
    ("target.system.crashdumps.description", "Speicherabbilder von Abstürzen und Bluescreens. Oft mehrere Gigabyte.", "Memory dumps from crashes and blue screens. Often several gigabytes."),
    ("target.system.logs.name", "Windows-Protokolle", "Windows logs"),
    ("target.system.logs.description", "Ältere Protokolldateien von Updates und Systemwartung. Die aktive Datei bleibt.", "Older log files from updates and servicing. The active file is kept."),
    ("target.system.dns.name", "DNS-Zwischenspeicher", "DNS cache"),
    ("target.system.dns.description", "Leert die zwischengespeicherten Namensauflösungen. Hilft bei hängenden Verbindungen, gibt aber keinen Speicher frei.", "Flushes cached name lookups. Helps with stuck connections but frees no space."),
    ("target.system.prefetch.name", "Prefetch-Daten", "Prefetch data"),
    ("target.system.prefetch.description", "Startbeschleunigung von Windows. Wird neu aufgebaut – Programme starten übergangsweise langsamer.", "Windows' startup accelerator. It is rebuilt – programs start more slowly for a while."),
    ("target.system.windowsupdate.name", "Windows-Update-Zwischenspeicher", "Windows Update cache"),
    ("target.system.windowsupdate.description", "Bereits installierte Updatepakete. Die Update-Historie bleibt erhalten.", "Update packages that are already installed. The update history is kept."),
    ("target.system.deliveryoptimization.name", "Übermittlungsoptimierung", "Delivery Optimization"),
    ("target.system.deliveryoptimization.description", "Zwischengespeicherte Update-Teile, die Windows an andere Geräte im Netz weitergibt.", "Cached update chunks Windows shares with other devices on your network."),
    ("target.system.fontcache.name", "Schriftcache", "Font cache"),
    ("target.system.fontcache.description", "Zwischenspeicher der Schriftdarstellung. Nach dem Leeren empfiehlt sich ein Neustart.", "Cache for font rendering. A restart is recommended afterwards."),
    ("target.system.recentdocs.name", "Zuletzt verwendete Dokumente", "Recent documents"),
    ("target.system.recentdocs.description", "Die Liste zuletzt geöffneter Dateien. Die Dateien selbst bleiben unberührt.", "The list of recently opened files. The files themselves are untouched."),
    ("target.system.searchcache.name", "Suche-Symbolcache", "Search icon cache"),
    ("target.system.searchcache.description", "Symbolzwischenspeicher der Windows-Suche.", "Icon cache of Windows Search."),
    ("target.system.windowsold.name", "Windows.old", "Windows.old"),
    ("target.system.windowsold.description", "Die vorherige Windows-Installation. Nach dem Löschen ist keine Rückkehr zur alten Version mehr möglich.", "Your previous Windows installation. Once deleted you cannot roll back to the old version."),

    // --- Ziele: Papierkorb ----------------------------------------------
    ("target.recyclebin.all.name", "Papierkorb leeren", "Empty Recycle Bin"),
    ("target.recyclebin.all.description", "Entfernt den Inhalt auf allen Laufwerken endgültig. Genau hier findet man versehentlich Gelöschtes wieder.", "Permanently removes the contents on all drives. This is exactly where accidentally deleted files are recovered from."),

    // --- Ziele: Browser --------------------------------------------------
    ("target.browser.edge.cache.name", "Microsoft Edge – Zwischenspeicher", "Microsoft Edge – cache"),
    ("target.browser.edge.cache.description", "Zwischengespeicherte Webinhalte. Anmeldungen und Verlauf bleiben erhalten.", "Cached web content. Logins and history are kept."),
    ("target.browser.chrome.cache.name", "Google Chrome – Zwischenspeicher", "Google Chrome – cache"),
    ("target.browser.chrome.cache.description", "Zwischengespeicherte Webinhalte. Anmeldungen und Verlauf bleiben erhalten.", "Cached web content. Logins and history are kept."),
    ("target.browser.firefox.cache.name", "Mozilla Firefox – Zwischenspeicher", "Mozilla Firefox – cache"),
    ("target.browser.firefox.cache.description", "Zwischengespeicherte Webinhalte. Anmeldungen und Verlauf bleiben erhalten.", "Cached web content. Logins and history are kept."),
    ("target.browser.brave.cache.name", "Brave – Zwischenspeicher", "Brave – cache"),
    ("target.browser.brave.cache.description", "Zwischengespeicherte Webinhalte. Anmeldungen und Verlauf bleiben erhalten.", "Cached web content. Logins and history are kept."),
    ("target.browser.opera.cache.name", "Opera – Zwischenspeicher", "Opera – cache"),
    ("target.browser.opera.cache.description", "Zwischengespeicherte Webinhalte, auch für Opera GX. Anmeldungen bleiben erhalten.", "Cached web content, including Opera GX. Logins are kept."),
    ("target.browser.vivaldi.cache.name", "Vivaldi – Zwischenspeicher", "Vivaldi – cache"),
    ("target.browser.vivaldi.cache.description", "Zwischengespeicherte Webinhalte. Anmeldungen und Verlauf bleiben erhalten.", "Cached web content. Logins and history are kept."),
    ("target.browser.cookies.name", "Cookies aller Browser", "Cookies in all browsers"),
    ("target.browser.cookies.description", "Sie werden anschließend auf allen Websites abgemeldet.", "You will be signed out of every website afterwards."),
    ("target.browser.history.name", "Browserverlauf", "Browsing history"),
    ("target.browser.history.description", "Besuchte Seiten und Adressvervollständigung. Lesezeichen bleiben erhalten.", "Visited pages and address bar suggestions. Bookmarks are kept."),
    ("target.browser.predictor.name", "Vorhersagedaten", "Prediction data"),
    ("target.browser.predictor.description", "Statistiken, mit denen Browser Ihre nächste Eingabe erraten.", "Statistics browsers use to guess what you type next."),

    // --- Ziele: Anwendungen ---------------------------------------------
    ("target.app.discord.name", "Discord", "Discord"),
    ("target.app.discord.description", "Bild- und Codezwischenspeicher. Die Anmeldung bleibt bestehen.", "Image and code cache. You stay signed in."),
    ("target.app.teams.name", "Microsoft Teams", "Microsoft Teams"),
    ("target.app.teams.description", "Zwischenspeicher und Protokolle, klassisches und neues Teams.", "Cache and logs, both classic and new Teams."),
    ("target.app.spotify.name", "Spotify", "Spotify"),
    ("target.app.spotify.description", "Zwischenspeicher und Protokolle. Heruntergeladene Titel bleiben erhalten.", "Cache and logs. Downloaded tracks are kept."),
    ("target.app.vscode.name", "Visual Studio Code", "Visual Studio Code"),
    ("target.app.vscode.description", "Zwischenspeicher und Protokolle. Der lokale Dateiverlauf bleibt erhalten.", "Cache and logs. The local file history is kept."),
    ("target.app.gpushadercache.name", "Grafik-Shadercache", "GPU shader cache"),
    ("target.app.gpushadercache.description", "Vorkompilierte Grafikbausteine von NVIDIA, AMD, Intel und DirectX. Spiele laden einmalig länger.", "Precompiled graphics shaders from NVIDIA, AMD, Intel and DirectX. Games load slower once."),
    ("target.app.steam.name", "Steam", "Steam"),
    ("target.app.steam.description", "Web- und Protokollzwischenspeicher. Spiele und Spielstände bleiben unberührt.", "Web and log cache. Games and saves are untouched."),
    ("target.app.adobe.name", "Adobe Creative Cloud", "Adobe Creative Cloud"),
    ("target.app.adobe.description", "Medienzwischenspeicher. Premiere und After Effects müssen Material danach neu analysieren.", "Media cache. Premiere and After Effects have to re-conform footage afterwards."),
    ("target.app.java.name", "Java", "Java"),
    ("target.app.java.description", "Zwischenspeicher heruntergeladener Java-Anwendungen.", "Cache of downloaded Java applications."),
    ("target.app.office.name", "Microsoft Office", "Microsoft Office"),
    ("target.app.office.description", "Diagnoseprotokolle und Webdienstzwischenspeicher. Nicht hochgeladene Dokumentänderungen bleiben unberührt.", "Diagnostic logs and web service cache. Pending document changes are untouched."),
    ("target.app.packagemanagers.name", "Entwicklerwerkzeuge (npm, pip)", "Developer tools (npm, pip)"),
    ("target.app.packagemanagers.description", "Paketzwischenspeicher. Nächste Installationen laden erneut aus dem Netz.", "Package caches. The next install downloads from the network again."),

    // --- Ziele: Installationsdateien ------------------------------------
    ("target.installers.downloads.name", "Alte Installationsdateien", "Old installer files"),
    ("target.installers.downloads.description", "Setups in Downloads und auf dem Schreibtisch, älter als 30 Tage und größer als 5 MB. Plane löscht hier nie von selbst – Sie wählen einzeln aus.", "Setups in Downloads and on the desktop, older than 30 days and larger than 5 MB. Plane never deletes these on its own – you pick them individually."),

    // --- Ziele: Registry -------------------------------------------------
    ("target.registry.privacy.name", "Registry-Spuren", "Registry traces"),
    ("target.registry.privacy.description", "Zuletzt-gestartet-Listen. Eine Datenschutzfunktion – sie macht den Rechner nicht schneller.", "Recently-run lists. A privacy feature – it does not make your PC faster."),
    ("target.registry.orphans.name", "Verwaiste Registry-Einträge", "Orphaned registry entries"),
    ("target.registry.orphans.description", "Verweise auf Programme, die es nicht mehr gibt. Microsoft rät von Registry-Bereinigung ab; Plane sichert deshalb vorher automatisch.", "References to programs that no longer exist. Microsoft advises against registry cleaning, so Plane always creates a backup first."),

    // --- CLI -------------------------------------------------------------
    ("cli.description", "Plane – schlanker PC-Cleaner für Windows", "Plane – lightweight PC cleaner for Windows"),
    ("cli.scanning", "Analysiere", "Analyzing"),
    ("cli.no_selection", "Keine Ziele ausgewählt.", "No targets selected."),
    ("cli.total", "Gesamt", "Total"),
    ("cli.aborted", "Abgebrochen.", "Aborted."),
    ("cli.confirm_prompt", "Wirklich bereinigen? [j/N] ", "Really clean? [y/N] "),
    ("cli.confirm_yes", "j", "y"),
];

/// Übersetzten Text nachschlagen. Unbekannte Schlüssel geben sich selbst
/// zurück – das macht fehlende Übersetzungen in der Oberfläche sichtbar,
/// statt sie durch leere Felder zu verstecken.
pub fn t(lang: &str, key: &str) -> String {
    let index = if lang.eq_ignore_ascii_case("de") {
        1
    } else {
        2
    };
    for (schluessel, de, en) in ENTRIES {
        if *schluessel == key {
            return if index == 1 {
                de.to_string()
            } else {
                en.to_string()
            };
        }
    }
    key.to_string()
}

/// Text mit Platzhaltern `{0}`, `{1}`, … füllen.
pub fn format(lang: &str, key: &str, args: &[&str]) -> String {
    let mut text = t(lang, key);
    for (index, wert) in args.iter().enumerate() {
        text = text.replace(&format!("{{{index}}}"), wert);
    }
    text
}

/// Alle Texte einer Sprache – für das Frontend.
pub fn catalog(lang: &str) -> BTreeMap<String, String> {
    let de = lang.eq_ignore_ascii_case("de");
    ENTRIES
        .iter()
        .map(|(key, d, e)| {
            (
                (*key).to_string(),
                if de {
                    (*d).to_string()
                } else {
                    (*e).to_string()
                },
            )
        })
        .collect()
}

/// `true`, wenn die Sprache unterstützt wird.
pub fn is_supported(lang: &str) -> bool {
    LANGUAGES.iter().any(|(k, _)| k.eq_ignore_ascii_case(lang))
}

/// Sprache normalisieren: `de-DE` → `de`, Unbekanntes → [`FALLBACK`].
pub fn normalize(lang: &str) -> String {
    let basis = lang
        .split(['-', '_'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    if is_supported(&basis) {
        basis
    } else {
        FALLBACK.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{catalog as ziele, types::Category};

    #[test]
    fn schluessel_sind_eindeutig() {
        let mut gesehen: Vec<&str> = Vec::new();
        for (key, _, _) in ENTRIES {
            assert!(!gesehen.contains(key), "Doppelter Schlüssel: {key}");
            gesehen.push(key);
        }
    }

    #[test]
    fn kein_text_ist_leer() {
        for (key, de, en) in ENTRIES {
            assert!(!de.trim().is_empty(), "Deutsch fehlt: {key}");
            assert!(!en.trim().is_empty(), "Englisch fehlt: {key}");
        }
    }

    #[test]
    fn jedes_reinigungsziel_ist_uebersetzt() {
        for target in ziele::TARGETS {
            for schluessel in [target.i18n_name(), target.i18n_description()] {
                assert_ne!(
                    t("de", &schluessel),
                    schluessel,
                    "Deutsche Übersetzung fehlt: {schluessel}"
                );
                assert_ne!(
                    t("en", &schluessel),
                    schluessel,
                    "Englische Übersetzung fehlt: {schluessel}"
                );
            }
        }
    }

    #[test]
    fn jede_kategorie_ist_uebersetzt() {
        for kategorie in Category::ALL {
            let key = kategorie.i18n_key();
            assert_ne!(t("de", &key), key);
            assert_ne!(t("en", &key), key);
        }
    }

    #[test]
    fn keine_verwaisten_zieluebersetzungen() {
        // Umgekehrte Richtung: keine Übersetzung ohne zugehöriges Ziel.
        for (key, _, _) in ENTRIES {
            let Some(rest) = key.strip_prefix("target.") else {
                continue;
            };
            let ziel_key = rest
                .strip_suffix(".name")
                .or_else(|| rest.strip_suffix(".description"))
                .unwrap_or(rest);
            assert!(
                ziele::target_by_key(ziel_key).is_some(),
                "Übersetzung ohne Ziel: {key}"
            );
        }
    }

    #[test]
    fn deutsch_und_englisch_unterscheiden_sich_meistens() {
        // Identische Texte sind erlaubt (Eigennamen), aber selten.
        let gleich = ENTRIES.iter().filter(|(_, de, en)| de == en).count();
        assert!(
            gleich * 4 < ENTRIES.len(),
            "Verdächtig viele unübersetzte Einträge: {gleich} von {}",
            ENTRIES.len()
        );
    }

    #[test]
    fn unbekannter_schluessel_gibt_sich_selbst_zurueck() {
        assert_eq!(t("de", "gibt.es.nicht"), "gibt.es.nicht");
    }

    #[test]
    fn unbekannte_sprache_faellt_auf_englisch_zurueck() {
        assert_eq!(t("fr", "nav.settings"), t("en", "nav.settings"));
    }

    #[test]
    fn deutsch_nutzt_echte_umlaute() {
        // Kein "ae"/"oe"/"ue"-Ersatz mehr in der Oberfläche.
        const UMLAUTE: [char; 7] = ['ä', 'ö', 'ü', 'ß', 'Ä', 'Ö', 'Ü'];
        assert!(
            t("de", "nav.dashboard").contains(UMLAUTE),
            "erwartet einen Umlaut in: {}",
            t("de", "nav.dashboard")
        );
        assert!(t("de", "settings.title").contains("Einstellungen"));
        let mit_umlauten = ENTRIES
            .iter()
            .filter(|(_, de, _)| de.contains(['ä', 'ö', 'ü', 'ß', 'Ä', 'Ö', 'Ü']))
            .count();
        assert!(mit_umlauten > 30, "nur {mit_umlauten} Texte mit Umlauten");
    }

    #[test]
    fn platzhalter_werden_ersetzt() {
        assert_eq!(
            format("de", "home.selected_size", &["1,2 GB"]),
            "1,2 GB ausgewählt"
        );
        assert_eq!(
            format("en", "home.found_total", &["3 GB", "42"]),
            "3 GB found across 42 items"
        );
    }

    #[test]
    fn platzhalter_ohne_argumente_bleiben_stehen() {
        assert!(format("de", "home.selected_size", &[]).contains("{0}"));
    }

    #[test]
    fn katalog_enthaelt_alle_eintraege() {
        let de = catalog("de");
        assert_eq!(de.len(), ENTRIES.len());
        assert_eq!(catalog("en").len(), ENTRIES.len());
        assert_ne!(de["nav.settings"], catalog("en")["nav.settings"]);
    }

    #[test]
    fn sprachen_werden_normalisiert() {
        assert_eq!(normalize("de-DE"), "de");
        assert_eq!(normalize("de_AT"), "de");
        assert_eq!(normalize("DE"), "de");
        assert_eq!(normalize("en-GB"), "en");
        assert_eq!(normalize("fr-FR"), FALLBACK);
        assert_eq!(normalize(""), FALLBACK);
    }

    #[test]
    fn unterstuetzte_sprachen_sind_abfragbar() {
        assert!(is_supported("de"));
        assert!(is_supported("EN"));
        assert!(!is_supported("fr"));
        assert_eq!(LANGUAGES.len(), 2);
    }

    #[test]
    fn platzhalter_sind_in_beiden_sprachen_gleich() {
        for (key, de, en) in ENTRIES {
            for index in 0..3 {
                let muster = format!("{{{index}}}");
                assert_eq!(
                    de.contains(&muster),
                    en.contains(&muster),
                    "Platzhalter {muster} fehlt in einer Sprache: {key}"
                );
            }
        }
    }
}
