/**
 * Plane Tweaker – eine kleine, geprüfte Auswahl an Windows-Einstellungen.
 *
 * Drei Dinge macht diese Ansicht bewusst anders als verbreitete Werkzeuge:
 *
 * 1. **Drei Zustände.** „Teilweise" ist ein eigener Zustand, kein „aus".
 *    Der Schalter steht dann unbestimmt, statt einen falschen Eindruck zu
 *    erwecken.
 * 2. **Zurücknehmen ist nicht Ausschalten.** „Aus" setzt den
 *    Windows-Standard, „Zurücknehmen" den Zustand, den Plane vorgefunden
 *    hat. Beides steht als eigener Knopf da.
 * 3. **Nebenwirkung immer sichtbar.** Jeder Punkt nennt, was er kostet –
 *    nicht nur, was er bringt.
 *
 * # Warum nur eine Zeile neu gezeichnet wird
 *
 * Ein `replaceChildren` auf der ganzen Liste leert den Container für einen
 * Augenblick. Die Seite wird dabei kurz kürzer als die Bildlaufposition, der
 * Browser klemmt sie auf null – und die Ansicht springt nach oben. Wer einen
 * Punkt weiter unten umschaltet, verliert also seine Stelle. Deshalb hält
 * [`zeilen`] eine Zuordnung Schlüssel → Element, und nach einer Änderung
 * wird genau ein Knoten ersetzt.
 */

import * as api from './api.js';
import { $, el, zeige, setzeText } from './dom.js';
import { t } from './i18n.js';
import { zustand } from './state.js';
import { frageNach } from './confirm.js';

/** Reihenfolge der Gruppen – entspricht `TweakGroup::ALL` im Backend. */
const GRUPPEN = ['privacy', 'explorer', 'taskbar', 'performance', 'system'];

let punkte = [];
let geladen = false;
let laeuft = false;
let melde = () => {};

/** Schlüssel → Zeilenelement, damit einzeln nachgezeichnet werden kann. */
const zeilen = new Map();

export function verdrahte({ meldung } = {}) {
    if (meldung) melde = meldung;
}

export async function lade(erzwinge = false) {
    if (geladen && !erzwinge) return;

    zeige($('tweaks-skeleton'), true);
    $('tweaks-list')?.replaceChildren();

    punkte = (await api.holeTweaks()) ?? [];
    geladen = true;

    zeige($('tweaks-skeleton'), false);
    zeichneListe();
}

// ---------------------------------------------------------------------------
// Darstellung
// ---------------------------------------------------------------------------

export function beschrifte() {
    setzeText('tweaks-title', t('tweaks.title'));
    setzeText('tweaks-subtitle', t('tweaks.subtitle'));
    setzeText('tweaks-omitted', t('tweaks.omitted'));
    if (geladen) zeichneListe();
}

/** Vollständiger Neuaufbau – nur beim Laden und beim Sprachwechsel. */
function zeichneListe() {
    const liste = $('tweaks-list');
    if (!liste) return;

    zeilen.clear();

    const gruppen = GRUPPEN.map((gruppe) => {
        const eintraege = punkte.filter((p) => p.group === gruppe);
        return eintraege.length > 0 ? baueGruppe(gruppe, eintraege) : null;
    }).filter(Boolean);

    liste.replaceChildren(...gruppen);
}

/** Eine einzelne Zeile an Ort und Stelle austauschen. */
function zeichneZeile(punkt) {
    const alt = zeilen.get(punkt.key);
    const neu = baueZeile(punkt);
    if (alt?.isConnected) {
        alt.replaceWith(neu);
    }
}

function baueGruppe(gruppe, eintraege) {
    return el('section', { klasse: 'tweak-group', dataset: { gruppe } }, [
        el('h2', { klasse: 'tweak-group-title', text: t(`tweakgroup.${gruppe}`) }),
        el('div', { klasse: 'tweak-rows' }, eintraege.map(baueZeile)),
    ]);
}

/** Zustandsabzeichen – dieselbe Sprache wie in der Kommandozeile. */
function zustandsText(zustand_) {
    return t(`tweaks.state.${zustand_}`);
}

/**
 * Braucht dieser Punkt Rechte, die Plane gerade nicht hat?
 *
 * Der Punkt bleibt trotzdem bedienbar: ein Klick fragt nach einem Neustart
 * mit Administratorrechten, statt einfach nichts zu tun.
 */
const fehlenRechte = (punkt) => punkt.requires_admin && !zustand.istAdmin;

function baueZeile(punkt) {
    // Gesperrt ist nur, was auch mit Rechten nichts brächte.
    const gesperrt = punkt.unsupported || punkt.managed || laeuft;

    const schalter = el('input', {
        klasse: 'tweak-check',
        type: 'checkbox',
        checked: punkt.state === 'on',
        disabled: gesperrt,
    });
    // „Teilweise" ist weder an noch aus – der unbestimmte Zustand ist das
    // einzige Bedienelement, das genau das ausdrückt.
    schalter.indeterminate = punkt.state === 'mixed';
    schalter.addEventListener('change', () => schalte(punkt, schalter.checked));

    const hinweise = [
        el('span', {
            klasse: `badge badge-state badge-state-${punkt.state}`,
            text: zustandsText(punkt.state),
        }),
        el('span', { klasse: 'badge', text: t(`tweaks.effect.${punkt.apply}`) }),
        fehlenRechte(punkt)
            ? el('span', { klasse: 'badge badge-admin', text: t('risk.needs_admin') })
            : null,
        punkt.unsupported
            ? el('span', { klasse: 'badge badge-muted', text: t('tweak.unsupported') })
            : null,
        punkt.managed
            ? el('span', { klasse: 'badge badge-muted', text: t('tweak.managed') })
            : null,
    ];

    const koerper = el('div', { klasse: 'tweak-body' }, [
        el('div', { klasse: 'tweak-head' }, [
            el('span', { klasse: 'tweak-name', text: t(`tweak.${punkt.key}.name`) }),
            el('span', { klasse: 'badges' }, hinweise),
        ]),
        el('p', {
            klasse: 'tweak-desc',
            text: t(`tweak.${punkt.key}.description`),
        }),
        el('p', {
            klasse: 'tweak-effect',
            text: t(`tweak.${punkt.key}.effect`),
        }),
        punkt.state === 'mixed'
            ? el('p', { klasse: 'tweak-note', text: t('tweaks.state.mixed.hint') })
            : null,
    ]);

    const zurueck = el('button', {
        klasse: 'btn-text',
        type: 'button',
        text: t('tweaks.revert'),
        disabled: gesperrt,
    });
    zurueck.addEventListener('click', () => nimmZurueck(punkt));

    const zeile = el('div', { klasse: 'tweak-row' + (gesperrt ? ' is-locked' : '') }, [
        el('label', { klasse: 'tweak-main' }, [
            el('span', { klasse: 'switch' }, [
                schalter,
                el('span', { klasse: 'switch-track', attr: { 'aria-hidden': 'true' } }),
            ]),
            koerper,
        ]),
        zurueck,
    ]);

    zeilen.set(punkt.key, zeile);
    return zeile;
}

// ---------------------------------------------------------------------------
// Administratorrechte
// ---------------------------------------------------------------------------

/**
 * Rechte besorgen, falls der Punkt sie braucht.
 *
 * Windows kennt keinen Weg, einen einzelnen Registry-Schreibzugriff
 * nachträglich zu erhöhen. Wer `HKEY_LOCAL_MACHINE` schreiben will, muss den
 * **Prozess** erhöht starten — deshalb bietet Plane hier einen Neustart an
 * und löst damit die UAC-Rückfrage von Windows aus.
 *
 * @returns {Promise<boolean>} `true`, wenn weitergemacht werden kann.
 */
async function rechteBesorgen(punkt) {
    if (!fehlenRechte(punkt)) return true;

    const freigabe = await frageNach({
        titel: t('admin.restart'),
        einleitung: t('tweak.needs_admin'),
        punkte: [
            {
                name: t(`tweak.${punkt.key}.name`),
                grund: t('risk.needs_admin'),
            },
        ],
        warnung: t('admin.restart_hint'),
        zustimmen: t('admin.restart'),
    });
    if (!freigabe) return false;

    // Ab hier übernimmt Windows: die UAC-Rückfrage erscheint, und bei
    // Zustimmung beendet sich dieser Prozess zugunsten des erhöhten.
    const ergebnis = await api.starteAlsAdminNeu();
    if (ergebnis === 'already') {
        // Sollte nicht vorkommen – dann stimmte nur die Anzeige nicht.
        zustand.istAdmin = true;
        return true;
    }
    if (!ergebnis) melde(t('admin.failed'), true);
    return false;
}

// ---------------------------------------------------------------------------
// Schalten
// ---------------------------------------------------------------------------

async function schalte(punkt, an) {
    if (laeuft) return;

    if (!(await rechteBesorgen(punkt))) {
        // Der Schalter steht jetzt falsch – zurück auf den echten Zustand.
        zeichneZeile(punkt);
        return;
    }

    laeuft = true;
    const neuerZustand = await api.setzeTweak(punkt.key, an);
    laeuft = false;

    abschliessen(punkt, neuerZustand);
}

async function nimmZurueck(punkt) {
    if (laeuft) return;
    if (!(await rechteBesorgen(punkt))) return;

    laeuft = true;
    const neuerZustand = await api.nimmTweakZurueck(punkt.key);
    laeuft = false;

    abschliessen(punkt, neuerZustand);
}

/**
 * Ergebnis übernehmen.
 *
 * Der gemeldete Zustand kommt aus einer erneuten Messung der Registry, nicht
 * aus dem, was Plane zu schreiben versucht hat. Steht dort etwas anderes als
 * erwartet, sieht man das hier sofort.
 */
function abschliessen(punkt, neuerZustand) {
    if (neuerZustand === null || neuerZustand === undefined) {
        // Die Fehlermeldung steht bereits auf dem Bildschirm; die Anzeige
        // stellt den zuletzt bekannten Zustand wieder her.
        zeichneZeile(punkt);
        return;
    }

    punkt.state = neuerZustand;
    zeichneZeile(punkt);

    const nachwirkung =
        punkt.apply === 'immediate' ? '' : ` ${t(`tweaks.effect.${punkt.apply}`)}`;
    melde(
        `${t(`tweak.${punkt.key}.name`)}: ${zustandsText(neuerZustand)}.${nachwirkung}`,
        false
    );
}

/** Nur für Tests und den Sprachwechsel: Zustand zurücksetzen. */
export function vergiss() {
    punkte = [];
    zeilen.clear();
    geladen = false;
}
