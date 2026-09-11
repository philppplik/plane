/**
 * Ablauf von Analyse und Bereinigung.
 *
 * Hier liegt die einzige Stelle, die `scan` und `clean` auslöst – und damit
 * auch die einzige, die den Fortschritt öffnet und wieder schließt.
 *
 * Warum die Bereinigung in mehrere Aufträge zerfällt: `only_paths` wirkt im
 * Backend **global** über den ganzen Auftrag. Ein gemeinsamer Auftrag aus
 * Vorschlagsziel (braucht Einzelpfade) und normalen Zielen (brauchen alle
 * Treffer) würde die normalen Ziele auf die Einzelpfade einschränken und
 * praktisch nichts löschen. Deshalb: ein Auftrag für alle normalen Ziele,
 * je ein weiterer für jedes Vorschlagsziel.
 */

import * as api from './api.js';
import * as progress from './progress.js';
import * as dashboard from './dashboard.js';
import { zeigeErgebnis } from './result.js';
import { freigabeEinholen } from './confirm.js';
import { t } from './i18n.js';
import { zustand, scanVon, istWaehlbar } from './state.js';

/** Wird von `main.js` gesetzt: Laufwerksanzeige auffrischen. */
let aktualisiereLaufwerk = async () => {};

export function verdrahte(haken) {
    aktualisiereLaufwerk = haken.laufwerk ?? aktualisiereLaufwerk;
}

/** Abbruch anfordern; das Backend beendet den laufenden Vorgang zeitnah. */
export function brichAb() {
    progress.markiereAbbruch();
    api.brichAb();
}

// ---------------------------------------------------------------------------
// Analyse
// ---------------------------------------------------------------------------

/** Gesamten Katalog analysieren. */
export async function analysiere() {
    if (zustand.laeuft) return;

    zustand.laeuft = 'scan';
    dashboard.zeichne();
    progress.oeffne('scan', brichAb);

    let bericht = null;
    try {
        bericht = await api.analysiere([]);
    } finally {
        zustand.laeuft = null;
        progress.schliesse();
    }

    if (bericht) {
        zustand.bericht = bericht;
        zustand.scans = new Map(bericht.targets.map((ziel) => [ziel.key, ziel]));
        stelleAuswahlHer();
    }

    dashboard.zeichne();
    await aktualisiereLaufwerk();
}

/**
 * Auswahl nach der Analyse setzen.
 *
 * Bevorzugt wird die zuletzt benutzte Auswahl aus den Einstellungen; gibt es
 * keine, greift die Empfehlung des Katalogs. Vorschlagsziele bleiben immer
 * abgewählt – sie verlangen eine Einzelauswahl.
 */
function stelleAuswahlHer() {
    const zuletzt = zustand.einstellungen?.selected_targets ?? [];
    zustand.auswahl.clear();

    for (const ziel of zustand.ziele) {
        if (!istWaehlbar(ziel) || ziel.suggestion_only) continue;
        const gewuenscht = zuletzt.length > 0 ? zuletzt.includes(ziel.key) : ziel.default_enabled;
        if (gewuenscht) zustand.auswahl.add(ziel.key);
    }
}

// ---------------------------------------------------------------------------
// Bereinigung
// ---------------------------------------------------------------------------

/** Ausgewählte Ziele bereinigen. */
export async function bereinige() {
    if (zustand.laeuft || zustand.auswahl.size === 0) return;

    const trockenlauf = Boolean(zustand.einstellungen?.dry_run_default);

    if (!(await freigabeFuerRiskante())) return;

    const auftraege = baueAuftraege();
    if (auftraege.length === 0) return;

    zustand.laeuft = 'clean';
    dashboard.zeichne();
    progress.oeffne('clean', brichAb);

    const berichte = [];
    try {
        for (const [index, auftrag] of auftraege.entries()) {
            progress.setzeAbschnitt(index, auftraege.length);
            const bericht = await api.bereinige(
                auftrag.targets,
                auftrag.only_paths,
                trockenlauf
            );
            if (!bericht) break;
            berichte.push(bericht);
            if (bericht.cancelled) break;
        }
    } finally {
        zustand.laeuft = null;
        progress.schliesse();
    }

    await merkeAuswahl();

    if (berichte.length > 0) zeigeErgebnis(fasseZusammen(berichte), trockenlauf);

    // Nach dem Löschen ist das Analyseergebnis überholt. Es stehen zu lassen
    // würde Größen versprechen, die es nicht mehr gibt.
    zustand.bericht = null;
    zustand.scans.clear();
    zustand.pfadAuswahl.clear();
    zustand.detailsOffen.clear();
    zustand.auswahl.clear();

    dashboard.zeichne();
    await aktualisiereLaufwerk();
}

/** Rückfrage bei Zielen der Stufe `caution`, sofern eingeschaltet. */
async function freigabeFuerRiskante() {
    if (!zustand.einstellungen?.confirm_risky) return true;

    const riskante = zustand.ziele
        .filter((ziel) => zustand.auswahl.has(ziel.key) && ziel.risk === 'caution')
        .map((ziel) => ({
            name: t(ziel.name_key),
            grund: t('risk.caution.hint'),
        }));

    return freigabeEinholen(riskante);
}

/** Auswahl in Aufträge zerlegen (siehe Modulkommentar). */
function baueAuftraege() {
    const normale = [];
    const auftraege = [];

    for (const key of zustand.auswahl) {
        const scan = scanVon(key);
        if (!scan) continue;

        if (!scan.suggestion_only) {
            normale.push(key);
            continue;
        }
        const pfade = [...(zustand.pfadAuswahl.get(key) ?? [])];
        if (pfade.length > 0) auftraege.push({ targets: [key], only_paths: pfade });
    }

    if (normale.length > 0) auftraege.unshift({ targets: normale, only_paths: [] });
    return auftraege;
}

/** Mehrere Teilberichte zu einem Gesamtbericht verschmelzen. */
function fasseZusammen(berichte) {
    return berichte.reduce((gesamt, bericht) => ({
        success: gesamt.success && bericht.success,
        targets: [...gesamt.targets, ...bericht.targets],
        total_freed: gesamt.total_freed + bericht.total_freed,
        total_removed: gesamt.total_removed + bericht.total_removed,
        duration_ms: gesamt.duration_ms + bericht.duration_ms,
        cancelled: gesamt.cancelled || bericht.cancelled,
        registry_backup: gesamt.registry_backup ?? bericht.registry_backup ?? null,
        error: [gesamt.error, bericht.error].filter(Boolean).join(' '),
    }), {
        success: true,
        targets: [],
        total_freed: 0,
        total_removed: 0,
        duration_ms: 0,
        cancelled: false,
        registry_backup: null,
        error: '',
    });
}

/** Getroffene Auswahl für den nächsten Start sichern. */
async function merkeAuswahl() {
    if (!zustand.einstellungen) return;
    zustand.einstellungen = {
        ...zustand.einstellungen,
        selected_targets: [...zustand.auswahl],
    };
    const gespeichert = await api.speichereEinstellungen(zustand.einstellungen);
    if (gespeichert) zustand.einstellungen = gespeichert;
}
