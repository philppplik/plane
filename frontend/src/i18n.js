/**
 * Übersetzungen.
 *
 * Das Frontend hält **keine** eigenen Sprachdateien. Der einzige Katalog liegt
 * im Rust-Backend (`src-tauri/src/i18n.rs`) und wird von Oberfläche und
 * Kommandozeile gemeinsam genutzt – so kann kein Text auseinanderlaufen.
 */

/** Aktuell geladener Katalog: Schlüssel → Text. */
let katalog = Object.create(null);

/** Aktuelles Sprachkürzel, u. a. für `Intl`-Formatierungen. */
let sprache = 'de';

/** Katalog nach einem Sprachwechsel austauschen. */
export function setzeKatalog(neuerKatalog, sprachkuerzel) {
    katalog = neuerKatalog ?? Object.create(null);
    if (sprachkuerzel) sprache = sprachkuerzel;
}

/** Aktives Sprachkürzel. */
export const aktiveSprache = () => sprache;

/**
 * Text nachschlagen und Platzhalter `{0}`, `{1}` … füllen.
 *
 * Unbekannte Schlüssel geben sich selbst zurück: eine fehlende Übersetzung
 * fällt so sofort auf, statt sich als leere Fläche zu verstecken.
 */
export function t(key, ...args) {
    let text = Object.prototype.hasOwnProperty.call(katalog, key) ? katalog[key] : key;
    args.forEach((wert, index) => {
        text = text.split(`{${index}}`).join(String(wert));
    });
    return text;
}

/**
 * Meldung der Engine auflösen.
 *
 * Das Backend liefert Gründe, Warnungen und Fehler entweder als reinen
 * Schlüssel (`"skip.needs_admin"`) oder als Schlüssel mit Argumenten
 * (`"warn.process_running|chrome.exe"`). Beides landet hier.
 */
export function tMeldung(rohtext) {
    if (!rohtext) return '';
    const teile = String(rohtext).split('|');
    return t(teile[0], ...teile.slice(1));
}

/**
 * Alle Knoten mit `data-i18n` befüllen.
 *
 * Für statische Beschriftungen im Markup – spart je Element eine Zeile
 * JavaScript und hält die Zuordnung im HTML sichtbar.
 */
export function uebersetzeMarkup(wurzel = document) {
    for (const knoten of wurzel.querySelectorAll('[data-i18n]')) {
        knoten.textContent = t(knoten.dataset.i18n);
    }
    for (const knoten of wurzel.querySelectorAll('[data-i18n-label]')) {
        knoten.setAttribute('aria-label', t(knoten.dataset.i18nLabel));
    }
}
