/**
 * Fortschrittsanzeige für Analyse und Bereinigung.
 *
 * Die Anzeige ist ein modales Overlay, weil während eines Laufs jede andere
 * Eingabe entweder wirkungslos oder gefährlich wäre – bis auf „Abbrechen“.
 *
 * Zwei Feinheiten, die den Balken ruhig halten:
 *
 * 1. Der Prozentwert darf innerhalb eines Laufs nie kleiner werden. Das
 *    Backend meldet pro Ziel Zwischenstände; ohne Sperre würde der Balken bei
 *    jedem Zielwechsel zurückspringen.
 * 2. Ein Lauf kann aus mehreren Aufrufen bestehen (siehe `run.js`). Über
 *    `setzeAbschnitt` wird der gemeldete Bereich 0–100 in ein Teilstück
 *    abgebildet, sodass der Balken über beide Aufrufe hinweg durchläuft.
 */

import { $, setzeText, zeige, fokusFalle } from './dom.js';
import { t } from './i18n.js';
import * as fmt from './format.js';

let falleAufheben = null;
let hoechsterWert = 0;
let abschnitt = { index: 0, anzahl: 1 };
let aufAbbruch = () => {};

/** Overlay öffnen. `phase` ist `"scan"` oder `"clean"`. */
export function oeffne(phase, abbruch) {
    aufAbbruch = abbruch ?? (() => {});
    hoechsterWert = 0;
    abschnitt = { index: 0, anzahl: 1 };

    setzeText('progress-title', t(phase === 'clean' ? 'home.cleaning' : 'home.analyzing'));
    setzeText('progress-target', '');
    setzeText('progress-counter', '');
    setzeText('progress-bytes', fmt.bytes(0));
    setzeText('progress-path', '');
    setzeBalken(0);

    const knopf = $('progress-cancel');
    if (knopf) {
        knopf.textContent = t('home.cancel');
        knopf.disabled = false;
    }

    const overlay = $('progress-overlay');
    zeige(overlay, true);
    falleAufheben = fokusFalle(overlay, () => aufAbbruch());
}

/**
 * Teilabschnitt festlegen, in den kommende Meldungen abgebildet werden.
 * Beispiel: Abschnitt 1 von 2 belegt die Prozentspanne 50–100.
 */
export function setzeAbschnitt(index, anzahl) {
    abschnitt = { index, anzahl: Math.max(1, anzahl) };
    hoechsterWert = (index / abschnitt.anzahl) * 100;
    setzeBalken(hoechsterWert);
}

/** Eine Fortschrittsmeldung des Backends verarbeiten. */
export function aktualisiere(meldung) {
    if (!meldung) return;

    const anteil = fmt.prozent(meldung.percent) / abschnitt.anzahl;
    const gesamt = (abschnitt.index / abschnitt.anzahl) * 100 + anteil;
    if (gesamt > hoechsterWert) {
        hoechsterWert = gesamt;
        setzeBalken(gesamt);
    }

    if (meldung.target_i18n) setzeText('progress-target', t(meldung.target_i18n));

    // `total` ist 0, wenn das Backend nur einen Einzelpfad meldet.
    if (meldung.total > 0) {
        setzeText(
            'progress-counter',
            `${fmt.zahl(Math.min(meldung.index + 1, meldung.total))} / ${fmt.zahl(meldung.total)}`
        );
    }

    setzeText('progress-bytes', fmt.bytes(meldung.bytes));

    const pfad = $('progress-path');
    if (pfad) {
        pfad.textContent = fmt.kuerzePfad(meldung.current_path ?? '');
        pfad.title = meldung.current_path ?? '';
    }
}

/** Overlay schließen und Fokus zurückgeben. */
export function schliesse() {
    zeige($('progress-overlay'), false);
    falleAufheben?.();
    falleAufheben = null;
}

/** Abbruchknopf sperren, sobald der Abbruch angefordert wurde. */
export function markiereAbbruch() {
    const knopf = $('progress-cancel');
    if (knopf) knopf.disabled = true;
}

function setzeBalken(wert) {
    const gerundet = fmt.prozent(wert);
    const fuellung = $('progress-fill');
    if (fuellung) fuellung.style.width = `${gerundet}%`;
    $('progress-bar')?.setAttribute('aria-valuenow', String(Math.round(gerundet)));
    setzeText('progress-percent', `${fmt.zahl(gerundet, gerundet < 100 ? 1 : 0)} %`);
}

/** Abbruchknopf verdrahten. Wird einmal beim Start aufgerufen. */
export function verdrahte() {
    $('progress-cancel')?.addEventListener('click', () => aufAbbruch());
}
