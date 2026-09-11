/**
 * Plane Uninstaller – installierte Programme ansehen und entfernen.
 *
 * Plane löscht hier **nichts selbst**. Es startet den Deinstaller, den der
 * Hersteller in der Registry hinterlegt hat, und prüft danach, ob der Eintrag
 * verschwunden ist. Alles andere wäre geraten.
 *
 * Geschützte Einträge (Laufzeitpakete, Treiber, Sicherheitssoftware, Plane
 * selbst) sind standardmäßig ausgeblendet. Sichtbar machen lassen sie sich –
 * entfernen nicht. Die Begründung steht an jedem Eintrag.
 */

import * as api from './api.js';
import * as fmt from './format.js';
import { $, el, zeige, setzeText } from './dom.js';
import { t } from './i18n.js';
import { frageNach } from './confirm.js';

/** Zuletzt geladene Liste; leer, solange nichts geladen wurde. */
let programme = [];

/** Erst nach dem ersten Öffnen der Ansicht laden – das dauert spürbar. */
let geladen = false;

/** Aktueller Filtertext, kleingeschrieben. */
let suche = '';

/** Geschützte Einträge mit anzeigen? */
let zeigeGeschuetzte = false;

/** Läuft gerade eine Deinstallation? Dann bleiben alle Knöpfe gesperrt. */
let laeuft = false;

/**
 * Symbolquelle → Bilddaten, oder `null` wenn es keins gibt.
 *
 * Der Zwischenspeicher hängt an der Quelle, nicht an der Kennung: mehrere
 * Einträge desselben Herstellers zeigen oft auf dieselbe Datei.
 */
const symbole = new Map();

/** Callback aus `main.js` für Erfolgs- und Fehlermeldungen. */
let melde = () => {};

// ---------------------------------------------------------------------------
// Aufbau
// ---------------------------------------------------------------------------

export function verdrahte({ meldung } = {}) {
    if (meldung) melde = meldung;

    $('programs-search')?.addEventListener('input', (e) => {
        suche = e.target.value.trim().toLowerCase();
        zeichneListe();
    });

    $('programs-show-protected')?.addEventListener('change', (e) => {
        zeigeGeschuetzte = e.target.checked;
        zeichneListe();
    });
}

/**
 * Liste vom Backend holen.
 *
 * @param {boolean} erzwinge Auch dann laden, wenn schon etwas da ist.
 */
export async function lade(erzwinge = false) {
    if (geladen && !erzwinge) return;

    zeige($('programs-skeleton'), true);
    $('programs-list')?.replaceChildren();

    programme = (await api.holeProgramme()) ?? [];
    geladen = true;

    zeige($('programs-skeleton'), false);
    zeichneListe();

    // Die Liste steht – die Bilder kommen nach. Sie aus demselben Aufruf zu
    // holen würde den ersten Eindruck um mehrere Sekunden verzögern.
    ladeSymbole().catch(() => {});
}

/**
 * Fehlende Symbole nachladen und die betroffenen Zeilen auffrischen.
 *
 * Gefragt wird nur nach dem, was noch nicht im Zwischenspeicher liegt.
 */
async function ladeSymbole() {
    const offen = [
        ...new Set(
            programme
                .map((p) => p.icon)
                .filter((quelle) => quelle && !symbole.has(quelle))
        ),
    ];
    if (offen.length === 0) return;

    const geholt = (await api.holeProgrammsymbole(offen)) ?? [];
    offen.forEach((quelle, index) => symbole.set(quelle, geholt[index] ?? null));

    zeichneSymbole();
}

/**
 * Die Bilder in die bereits gezeichneten Zeilen setzen.
 *
 * Die Liste wird **nicht** neu aufgebaut: das würde die Bildlaufposition auf
 * null klemmen und die Suche unterbrechen. Stattdessen bekommt jede
 * Symbolfläche ihren Inhalt einzeln.
 */
function zeichneSymbole() {
    for (const { quelle, flaeche } of flaechen) {
        const symbol = symbole.get(quelle);
        if (symbol && flaeche.isConnected) male(flaeche, symbol);
    }
}

/**
 * Gezeichnete Symbolflächen mit ihrer Quelle.
 *
 * Bewusst eine Liste und keine `Map`: mehrere Einträge desselben Herstellers
 * zeigen auf dieselbe Datei, und eine `Map` behielte davon nur einen.
 */
const flaechen = [];

/**
 * Rohe Bildpunkte auf eine Zeichenfläche bringen.
 *
 * Das Backend liefert RGBA statt PNG – das spart eine Kiste im Rust-Teil und
 * kommt ohne `data:`-Adressen aus, die an der Content-Security-Policy hängen
 * bleiben könnten.
 */
function male(flaeche, symbol) {
    const binaer = atob(symbol.rgba);
    const punkte = new Uint8ClampedArray(binaer.length);
    for (let i = 0; i < binaer.length; i += 1) punkte[i] = binaer.charCodeAt(i);

    const leinwand = el('canvas', {
        klasse: 'entry-icon-canvas',
        width: symbol.width,
        height: symbol.height,
    });

    const kontext = leinwand.getContext('2d');
    if (!kontext) return;
    kontext.putImageData(new ImageData(punkte, symbol.width, symbol.height), 0, 0);

    // Der Anfangsbuchstabe weicht dem echten Bild.
    flaeche.replaceChildren(leinwand);
    flaeche.classList.add('has-icon');
}

// ---------------------------------------------------------------------------
// Darstellung
// ---------------------------------------------------------------------------

/** Beschriftungen – wird auch nach einem Sprachwechsel gerufen. */
export function beschrifte() {
    setzeText('programs-title', t('uninstall.title'));
    setzeText('programs-subtitle', t('uninstall.subtitle'));
    setzeText('programs-search-label', t('uninstall.search'));
    setzeText('programs-protected-label', t('uninstall.show_protected'));

    const feld = $('programs-search');
    if (feld) feld.placeholder = t('uninstall.search');

    if (geladen) zeichneListe();
}

/** Sichtbare Teilmenge nach Suche und Schutzfilter. */
function gefiltert() {
    return programme.filter((p) => {
        if (!p.removable && !zeigeGeschuetzte) return false;
        if (!suche) return true;
        return (
            p.name.toLowerCase().includes(suche) ||
            (p.publisher ?? '').toLowerCase().includes(suche)
        );
    });
}

function zeichneListe() {
    const liste = $('programs-list');
    if (!liste) return;

    const sichtbar = gefiltert();
    const gesperrt = programme.filter((p) => !p.removable).length;
    setzeText('programs-count', t('uninstall.count', programme.length, gesperrt));

    if (sichtbar.length === 0) {
        liste.replaceChildren(
            el('p', { klasse: 'empty-note', text: t('uninstall.no_match') })
        );
        return;
    }

    flaechen.length = 0;
    liste.replaceChildren(...sichtbar.map(baueZeile));
}

/**
 * `YYYYMMDD` lesbar machen.
 *
 * Windows liefert das Feld ungeprüft aus der Registry; unplausible Werte
 * werden deshalb weggelassen statt falsch angezeigt.
 */
function datum(roh) {
    if (!/^\d{8}$/.test(roh ?? '')) return '';
    const jahr = Number(roh.slice(0, 4));
    const monat = Number(roh.slice(4, 6));
    const tag = Number(roh.slice(6, 8));
    if (jahr < 1990 || monat < 1 || monat > 12 || tag < 1 || tag > 31) return '';
    return new Date(jahr, monat - 1, tag).toLocaleDateString();
}

/**
 * Symbolfläche einer Zeile.
 *
 * Bleibt leer, wenn es kein Symbol gibt – der Platz wird trotzdem gehalten,
 * damit die Namen aller Zeilen auf derselben Linie stehen. Eine Liste, in
 * der jede zweite Zeile um 40 Punkte versetzt ist, liest sich schlecht.
 */
function baueSymbol(programm) {
    // Store-Pakete hinterlegen ihr Symbol im Paketmanifest statt in der
    // Registry; auch bei klassischen Programmen fehlt die Angabe manchmal.
    // Der Anfangsbuchstabe ist dann besser als eine leere Kachel: er
    // unterscheidet die Zeilen und wirkt nicht wie ein Ladefehler.
    const flaeche = el('div', {
        klasse: 'entry-icon',
        text: (programm.name.trim()[0] ?? '?').toUpperCase(),
        attr: { 'aria-hidden': 'true' },
    });

    if (programm.icon) {
        flaechen.push({ quelle: programm.icon, flaeche });
        const bereits = symbole.get(programm.icon);
        if (bereits) male(flaeche, bereits);
    }
    return flaeche;
}

function baueZeile(programm) {
    const abzeichen = el('span', { klasse: 'badges' }, [
        el('span', {
            klasse: 'badge badge-source',
            text: t(`uninstall.source.${programm.source}`),
        }),
        programm.requires_admin
            ? el('span', { klasse: 'badge badge-admin', text: t('uninstall.needs_admin') })
            : null,
        programm.removable && !programm.quiet
            ? el('span', { klasse: 'badge', text: t('uninstall.loud_hint') })
            : null,
    ]);

    const angaben = [
        programm.publisher,
        programm.version,
        datum(programm.install_date),
    ].filter(Boolean);

    const koerper = el('div', { klasse: 'entry-body' }, [
        el('div', { klasse: 'entry-head' }, [
            el('span', { klasse: 'entry-name', text: programm.name }),
            abzeichen,
        ]),
        angaben.length > 0
            ? el('p', { klasse: 'entry-meta', text: angaben.join(' · ') })
            : null,
        programm.protection
            ? el('p', { klasse: 'entry-note', text: t(programm.protection) })
            : null,
    ]);

    const groesse = el('span', {
        klasse: 'entry-size',
        text: programm.size > 0 ? fmt.bytes(programm.size) : '',
    });

    const knopf = el('button', {
        klasse: 'btn-secondary btn-small',
        type: 'button',
        text: t('uninstall.remove'),
        disabled: !programm.removable || laeuft,
    });
    knopf.addEventListener('click', () => entferne(programm, knopf));

    return el(
        'div',
        {
            klasse: 'entry' + (programm.removable ? '' : ' is-protected'),
            attr: { role: 'listitem' },
        },
        [baueSymbol(programm), koerper, groesse, programm.removable ? knopf : null]
    );
}

// ---------------------------------------------------------------------------
// Deinstallation
// ---------------------------------------------------------------------------

async function entferne(programm, knopf) {
    if (laeuft) return;

    const freigabe = await frageNach({
        titel: t('uninstall.remove'),
        einleitung: t('uninstall.confirm', programm.name),
        punkte: [
            {
                name: programm.name,
                grund: programm.quiet
                    ? t('uninstall.quiet_hint')
                    : t('uninstall.loud_hint'),
            },
        ],
        warnung: programm.requires_admin ? t('uninstall.needs_admin') : '',
        zustimmen: t('uninstall.remove'),
    });
    if (!freigabe) return;

    laeuft = true;
    knopf.disabled = true;
    knopf.textContent = t('uninstall.removing', programm.name);

    const ergebnis = await api.deinstalliere(programm, programm.quiet);

    laeuft = false;

    if (!ergebnis) {
        // Der Fehler steht bereits als Meldung auf dem Bildschirm.
        knopf.disabled = false;
        knopf.textContent = t('uninstall.remove');
        return;
    }

    melde(
        ergebnis.message
            ? t(ergebnis.message, programm.name)
            : t('uninstall.done', programm.name),
        !ergebnis.ok
    );

    // Neu laden statt die Zeile zu entfernen: nur die Registry weiß, ob der
    // Eintrag tatsächlich weg ist.
    await lade(true);
}

/** Nur für Tests und den Sprachwechsel: Zustand zurücksetzen. */
export function vergiss() {
    programme = [];
    geladen = false;
}
