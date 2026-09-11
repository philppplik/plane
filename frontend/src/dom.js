/**
 * Kleine DOM-Werkzeuge.
 *
 * Warum eigene Helfer statt direkter DOM-Aufrufe: die Oberfläche baut jedes
 * Element aus Daten auf. Mit `el()` bleibt das lesbar und – wichtiger – es
 * entsteht nie die Versuchung, `innerHTML` mit Werten aus dem Dateisystem zu
 * füllen. Pfadnamen dürfen kein Markup werden.
 */

/** Element nach ID. Bewusst ohne Fehlermeldung – Aufrufer prüfen selbst. */
export const $ = (id) => document.getElementById(id);

/**
 * Element bauen.
 *
 * `text` wird immer als Textknoten gesetzt, nie als HTML – das ist die
 * einzige Stelle, an der fremde Zeichenketten in die Seite gelangen.
 */
export function el(tag, optionen = {}, kinder = []) {
    const knoten = document.createElement(tag);
    const { text, klasse, attr, dataset, ...rest } = optionen;

    if (klasse) knoten.className = klasse;
    if (text !== undefined && text !== null) knoten.textContent = String(text);

    for (const [name, wert] of Object.entries(attr ?? {})) {
        if (wert === false || wert === null || wert === undefined) continue;
        knoten.setAttribute(name, wert === true ? '' : String(wert));
    }
    for (const [name, wert] of Object.entries(dataset ?? {})) {
        knoten.dataset[name] = String(wert);
    }
    Object.assign(knoten, rest);

    for (const kind of kinder) {
        if (kind) knoten.appendChild(kind);
    }
    return knoten;
}

/** Sichtbarkeit über das `hidden`-Attribut steuern. */
export function zeige(knoten, sichtbar) {
    if (!knoten) return;
    knoten.hidden = !sichtbar;
    knoten.classList.toggle('is-hidden', !sichtbar);
}

/** Textinhalt setzen, ohne bei fehlendem Element zu stolpern. */
export function setzeText(id, text) {
    const knoten = $(id);
    if (knoten) knoten.textContent = text ?? '';
}

/**
 * Tastaturbedienung für Elemente, die wie ein Knopf wirken.
 *
 * Enter und Leertaste müssen dasselbe auslösen wie ein Klick, sonst ist die
 * Zielliste ohne Maus nicht bedienbar.
 */
export function alsKnopf(knoten, aktion) {
    knoten.addEventListener('click', aktion);
    knoten.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            aktion(e);
        }
    });
}

/**
 * Fokus im Dialog einsperren.
 *
 * Ohne Fokusfalle springt der Tabulator hinter den Dialog und bedient
 * verdeckte Knöpfe – bei einem Löschwerkzeug ein echtes Risiko.
 * Gibt eine Funktion zurück, die die Falle wieder aufhebt.
 */
export function fokusFalle(dialog, beimSchliessen) {
    const vorher = document.activeElement;

    const fokussierbar = () =>
        [...dialog.querySelectorAll(
            'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
        )].filter((k) => !k.disabled && k.offsetParent !== null);

    const taste = (e) => {
        if (e.key === 'Escape') {
            e.preventDefault();
            beimSchliessen?.();
            return;
        }
        if (e.key !== 'Tab') return;

        const liste = fokussierbar();
        if (liste.length === 0) return;
        const erstes = liste[0];
        const letztes = liste[liste.length - 1];

        if (e.shiftKey && document.activeElement === erstes) {
            e.preventDefault();
            letztes.focus();
        } else if (!e.shiftKey && document.activeElement === letztes) {
            e.preventDefault();
            erstes.focus();
        }
    };

    document.addEventListener('keydown', taste, true);
    fokussierbar()[0]?.focus();

    return () => {
        document.removeEventListener('keydown', taste, true);
        if (vorher instanceof HTMLElement) vorher.focus();
    };
}
