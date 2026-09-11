/**
 * Zugriff auf die Tauri-Commands.
 *
 * Alle `invoke`-Aufrufe laufen durch diese Datei. Zwei Gründe: die Namen und
 * Argumente stehen an genau einer Stelle (der Vertragstest prüft sie dort),
 * und ein fehlgeschlagener Aufruf landet sichtbar in der Oberfläche statt
 * stumm in der Konsole.
 */

import { invoke } from '@tauri-apps/api/core';

/** Wird von `main.js` gesetzt und zeigt Fehler an. */
let fehlerAnzeige = () => {};

export function setzeFehlerAnzeige(anzeige) {
    fehlerAnzeige = anzeige;
}

/**
 * Command aufrufen und Fehler melden.
 *
 * Gibt `ersatz` zurück, wenn der Aufruf scheitert – so bleibt die Oberfläche
 * bedienbar, auch wenn eine einzelne Systemabfrage fehlschlägt.
 */
async function rufe(command, args, ersatz = null) {
    try {
        return await invoke(command, args);
    } catch (fehler) {
        fehlerAnzeige(`${command}: ${fehler}`);
        return ersatz;
    }
}

// --- Navigation ------------------------------------------------------------

export const holeStartbildschirm = () => rufe('get_start_screen', undefined, 'welcome');
export const starteApp = () => rufe('start_app', undefined, 'home');
export const zeigeUebersicht = () => rufe('show_home', undefined, 'home');
export const zeigeUeber = () => rufe('show_about', undefined, 'about');

// --- Sprache und Einstellungen --------------------------------------------

export const holeSprachen = () => rufe('get_languages', undefined, []);
export const holeUebersetzungen = (language) =>
    rufe('get_translations', { language }, {});
export const holeEinstellungen = () => rufe('get_settings', undefined, null);
export const speichereEinstellungen = (settings) =>
    rufe('set_settings', { settings }, settings);
export const setzeWillkommenZurueck = () => rufe('reset_welcome');

// --- Informationen ---------------------------------------------------------

export const holeMarke = () => rufe('get_brand', undefined, null);
export const holeLaufwerk = () => rufe('get_disk_stats', undefined, null);
export const holeSystem = () => rufe('get_system_info', undefined, null);
export const holeZiele = () => rufe('list_targets', undefined, []);

// --- Analyse und Bereinigung ----------------------------------------------

/** Leere Zielliste bedeutet für das Backend: gesamter Katalog. */
export const analysiere = (targets = []) => rufe('scan', { targets }, null);

export const bereinige = (targets, onlyPaths = [], dryRun = false) =>
    rufe('clean', { request: { targets, only_paths: onlyPaths, dry_run: dryRun } }, null);

export const brichAb = () => rufe('cancel_run');

// --- Programme -------------------------------------------------------------

export const holeProgramme = () => rufe('list_programs', undefined, []);

/** `quiet` versucht eine Deinstallation ohne Dialog des Herstellers. */
export const deinstalliere = (program, quiet) =>
    rufe('uninstall_program', { program, quiet }, null);

// --- Aktualisierungsprüfung ------------------------------------------------
//
// Der einzige Netzwerkzugriff der Anwendung. `pruefeAktualisierung` wird nur
// aufgerufen, wenn die Einstellung an ist oder der Nutzer ausdrücklich auf
// „Jetzt prüfen" drückt.

export const pruefeAktualisierung = () => rufe('check_update', undefined, null);
export const oeffneVeroeffentlichung = () => rufe('open_release_page');
export const ueberspringeVersion = (version) => rufe('skip_version', { version });

// --- Windows-Einstellungen -------------------------------------------------

export const holeTweaks = () => rufe('list_tweaks', undefined, []);
export const setzeTweak = (key, enabled) => rufe('set_tweak', { key, enabled }, null);
export const nimmTweakZurueck = (key) => rufe('revert_tweak', { key }, null);
