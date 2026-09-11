/**
 * Zahlen-, Größen- und Pfadaufbereitung.
 *
 * Alles hier ist bewusst sprachneutral aufgebaut: Einheitenzeichen (kB, MB, %)
 * sind in beiden Sprachen gleich, die Trennzeichen kommen aus `Intl`. Damit
 * braucht keine dieser Funktionen einen Übersetzungsschlüssel.
 */

import { aktiveSprache } from './i18n.js';

const EINHEITEN = ['B', 'kB', 'MB', 'GB', 'TB'];

/**
 * Bytes menschenlesbar machen.
 *
 * Basis 1024, weil Windows Speicherplatz genau so ausweist – eine Abweichung
 * würde die Anzeige neben dem Explorer unglaubwürdig machen.
 */
export function bytes(anzahl) {
    const wert = Number(anzahl) || 0;
    if (wert < 1) return `0 ${EINHEITEN[0]}`;

    let stufe = 0;
    let rest = wert;
    while (rest >= 1024 && stufe < EINHEITEN.length - 1) {
        rest /= 1024;
        stufe += 1;
    }
    const stellen = stufe === 0 ? 0 : rest < 10 ? 2 : rest < 100 ? 1 : 0;
    return `${zahl(rest, stellen)} ${EINHEITEN[stufe]}`;
}

/** Zahl mit landesüblichen Trennzeichen. */
export function zahl(wert, stellen = 0) {
    return new Intl.NumberFormat(aktiveSprache(), {
        minimumFractionDigits: stellen,
        maximumFractionDigits: stellen,
    }).format(Number(wert) || 0);
}

/** Prozentwert für Anzeige und `aria-valuenow`. */
export function prozent(wert) {
    const begrenzt = Math.min(100, Math.max(0, Number(wert) || 0));
    return Math.round(begrenzt * 10) / 10;
}

/**
 * Langen Pfad in der Mitte kürzen.
 *
 * Der Anfang (Laufwerk) und das Ende (Dateiname) sind die informativen Teile;
 * gekürzt wird deshalb dazwischen. So entstehen keine Zeilenumbrüche und keine
 * waagerechte Bildlaufleiste im schmalen Fenster.
 */
export function kuerzePfad(pfad, maximum = 72) {
    const text = String(pfad ?? '');
    if (text.length <= maximum) return text;
    const kopf = Math.ceil((maximum - 1) * 0.4);
    const fuss = Math.floor((maximum - 1) * 0.6);
    return `${text.slice(0, kopf)}…${text.slice(text.length - fuss)}`;
}

/** Dateiendung als sprachneutrale Typangabe (z. B. `.exe`). */
export function endung(pfad) {
    const name = String(pfad ?? '').split(/[\\/]/).pop() ?? '';
    const punkt = name.lastIndexOf('.');
    return punkt > 0 ? name.slice(punkt).toLowerCase() : '';
}

/** Dateiname ohne Verzeichnis. */
export function dateiname(pfad) {
    return String(pfad ?? '').split(/[\\/]/).pop() ?? '';
}
