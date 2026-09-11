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
 */

import * as api from './api.js';
import { $, el, zeige, setzeText } from './dom.js';
import { t } from './i18n.js';
import { frageNach } from './confirm.js';

/** Reihenfolge der Gruppen – entspricht `TweakGroup::ALL` im Backend. */
const GRUPPEN = ['privacy', 'explorer', 'taskbar', 'performance', 'system'];

let punkte = [];
let geladen = false;
let laeuft = false;
let melde = () => {};

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

function zeichneListe() {
    const liste = $('tweaks-list');
    if (!liste) return;

    const gruppen = GRUPPEN.map((gruppe) => {
        const eintraege = punkte.filter((p) => p.group === gruppe);
        return eintraege.length > 0 ? baueGruppe(gruppe, eintraege) : null;
    }).filter(Boolean);

    liste.replaceChildren(...gruppen);
}

function baueGruppe(gruppe, eintraege) {
    return el('section', { klasse: 'tweak-group', dataset: { gruppe } }, [
        el('h2', { klasse: 'tweak-group-title', text: t(`tweakgroup.${gruppe}`) }),
        el('div', { klasse: 'tweak-rows' }, eintraege.map(baueZeile)),
    ]);
}

/** Zustandsabzeichen – dieselbe Sprache wie in der Kommandozeile. */
function zustandsText(zustand) {
    return t(`tweaks.state.${zustand}`);
}

function baueZeile(punkt) {
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
        punkt.requires_admin
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

    return el('div', { klasse: 'tweak-row' + (gesperrt ? ' is-locked' : '') }, [
        el('label', { klasse: 'tweak-main' }, [
            el('span', { klasse: 'switch' }, [
                schalter,
                el('span', { klasse: 'switch-track', attr: { 'aria-hidden': 'true' } }),
            ]),
            koerper,
        ]),
        zurueck,
    ]);
}

// ---------------------------------------------------------------------------
// Schalten
// ---------------------------------------------------------------------------

async function schalte(punkt, an) {
    if (laeuft) return;

    if (punkt.requires_admin) {
        const freigabe = await frageNach({
            titel: t(`tweak.${punkt.key}.name`),
            einleitung: t(`tweak.${punkt.key}.effect`),
            warnung: t('risk.needs_admin'),
            zustimmen: an ? t('tweaks.apply') : t('tweaks.revert'),
        });
        if (!freigabe) {
            zeichneListe();
            return;
        }
    }

    laeuft = true;
    const zustand = await api.setzeTweak(punkt.key, an);
    laeuft = false;

    abschliessen(punkt, zustand);
}

async function nimmZurueck(punkt) {
    if (laeuft) return;

    laeuft = true;
    const zustand = await api.nimmTweakZurueck(punkt.key);
    laeuft = false;

    abschliessen(punkt, zustand);
}

/**
 * Ergebnis übernehmen.
 *
 * Der gemeldete Zustand kommt aus einer erneuten Messung der Registry, nicht
 * aus dem, was Plane zu schreiben versucht hat. Steht dort etwas anderes als
 * erwartet, sieht man das hier sofort.
 */
function abschliessen(punkt, zustand) {
    if (zustand === null || zustand === undefined) {
        // Die Fehlermeldung steht bereits auf dem Bildschirm; die Anzeige
        // stellt den zuletzt bekannten Zustand wieder her.
        zeichneListe();
        return;
    }

    punkt.state = zustand;
    zeichneListe();

    const nachwirkung = punkt.apply === 'immediate' ? '' : ` ${t(`tweaks.effect.${punkt.apply}`)}`;
    melde(`${t(`tweak.${punkt.key}.name`)}: ${zustandsText(zustand)}.${nachwirkung}`, false);
}

/** Nur für Tests und den Sprachwechsel: Zustand zurücksetzen. */
export function vergiss() {
    punkte = [];
    geladen = false;
}
