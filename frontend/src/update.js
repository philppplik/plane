/**
 * Hinweis auf eine neuere Version.
 *
 * Der einzige Teil der Oberfläche, hinter dem ein Netzwerkzugriff steht.
 * Deshalb gelten hier strengere Regeln als sonst:
 *
 * - **Nur wenn eingeschaltet.** `check_updates` ist standardmäßig aus. Ohne
 *   diese Einstellung wird beim Start nichts abgefragt.
 * - **Kein Selbstinstallieren.** Der Knopf öffnet die Veröffentlichungsseite
 *   im Browser. Herunterladen und Starten bleibt eine Handlung des Nutzers —
 *   solange die Pakete nicht signiert sind, wäre alles andere fahrlässig.
 * - **Kein Dauerhinweis.** „Diese Version überspringen" merkt sich die
 *   Versionsnummer. Ein Hinweis, den man nicht loswird, wird übersehen.
 * - **Stiller Fehlschlag beim Start.** Wer offline ist, hat kein Problem,
 *   das eine Fehlermeldung lösen würde. Nur die ausdrückliche Prüfung in den
 *   Einstellungen meldet Fehler.
 */

import * as api from './api.js';
import { $, zeige, setzeText } from './dom.js';
import { t } from './i18n.js';
import { zustand } from './state.js';

/** Zuletzt gefundene Version – für „überspringen". */
let gefunden = null;

let melde = () => {};

export function verdrahte({ meldung } = {}) {
    if (meldung) melde = meldung;

    $('update-download')?.addEventListener('click', oeffne);
    $('update-later')?.addEventListener('click', () => zeigeBanner(false));
    $('update-skip')?.addEventListener('click', ueberspringe);
}

// ---------------------------------------------------------------------------
// Prüfen
// ---------------------------------------------------------------------------

/**
 * Prüfung beim Start.
 *
 * Tut nichts, wenn die Einstellung aus ist. Fehler bleiben stumm: beim Start
 * hat der Nutzer nicht um diese Auskunft gebeten.
 */
export async function pruefeBeimStart() {
    if (!zustand.einstellungen?.check_updates) return;

    const info = await api.pruefeAktualisierung();
    if (!info || !info.newer) return;
    if (info.latest === zustand.einstellungen.skipped_version) return;

    gefunden = info;
    zeichneBanner(info);
    zeigeBanner(true);
}

/**
 * Prüfung auf ausdrücklichen Wunsch, aus den Einstellungen heraus.
 *
 * Hier wird jedes Ergebnis gemeldet — auch „alles aktuell" und auch Fehler.
 * Wer den Knopf drückt, wartet auf eine Antwort.
 */
export async function pruefeJetzt() {
    const anzeige = $('settings-update-result');
    const knopf = $('settings-check-now');

    if (anzeige) anzeige.textContent = t('update.checking');
    if (knopf) knopf.disabled = true;

    let info = null;
    try {
        info = await api.pruefeAktualisierung();
    } finally {
        if (knopf) knopf.disabled = false;
    }

    if (!info) {
        // Der Fehler steht bereits als Meldung auf dem Bildschirm; hier nur
        // das „wird geprüft" wieder wegräumen.
        if (anzeige) anzeige.textContent = '';
        return;
    }

    if (info.newer) {
        gefunden = info;
        zeichneBanner(info);
        zeigeBanner(true);
        if (anzeige) anzeige.textContent = t('update.available', info.latest, info.current);
    } else if (anzeige) {
        anzeige.textContent = `${t('update.up_to_date')} ${t('update.current', info.current)}`;
    }
}

// ---------------------------------------------------------------------------
// Banner
// ---------------------------------------------------------------------------

function zeichneBanner(info) {
    setzeText('update-banner-title', t('update.title'));
    setzeText('update-banner-detail', t('update.available', info.latest, info.current));
    setzeText(
        'update-banner-hint',
        info.published ? t('update.published', info.published) : t('update.install_hint')
    );
    setzeText('update-download', t('update.download'));
    setzeText('update-later', t('update.later'));
    setzeText('update-skip', t('update.skip'));
}

function zeigeBanner(sichtbar) {
    zeige($('update-banner'), sichtbar);
}

async function oeffne() {
    await api.oeffneVeroeffentlichung();
    // Der Browser übernimmt; das Banner hat seinen Zweck erfüllt.
    zeigeBanner(false);
}

async function ueberspringe() {
    if (!gefunden) return;
    await api.ueberspringeVersion(gefunden.latest);
    zustand.einstellungen.skipped_version = gefunden.latest;
    zeigeBanner(false);
}

/** Beschriftungen nach einem Sprachwechsel. */
export function beschrifte() {
    setzeText('settings-check-updates-label', t('settings.check_updates'));
    setzeText('settings-check-updates-hint', t('settings.check_updates.hint'));
    setzeText('settings-check-now', t('update.check_now'));
    // Ein Ergebnis in der alten Sprache stehen zu lassen wäre schlechter als
    // gar keins.
    setzeText('settings-update-result', '');
    if (gefunden) zeichneBanner(gefunden);
}
