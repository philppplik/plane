/**
 * Übersicht: Zielliste, Auswahl und Zusammenfassung.
 *
 * Die Liste ist nach Kategorien gruppiert, weil der Katalog über dreißig Ziele
 * kennt – flach gelistet wäre er unbedienbar. Jede Kategorie hat eine eigene
 * Sammelbox mit drei Zuständen (alle / teilweise / keine); der unbestimmte
 * Zustand ist der wichtigste, weil er Teilauswahlen sichtbar macht, ohne sie
 * beim Anklicken zu verlieren.
 *
 * Neu gezeichnet wird immer die **ganze** Liste. Bei dieser Größenordnung ist
 * das schnell genug und erspart eine fehleranfällige Teilaktualisierung.
 */

import { $, el, zeige, setzeText } from './dom.js';
import { t, tMeldung } from './i18n.js';
import * as fmt from './format.js';
import { schliesse as schliesseErgebnis } from './result.js';
import {
    zustand,
    scanVon,
    istWaehlbar,
    ausgewaehlteGroesse,
    ausgewaehlteAnzahl,
    summePfade,
    pfadeVon,
    setzeAuswahl,
    zieleDerKategorie,
} from './state.js';

/** Reihenfolge wie im Backend-Katalog (`Category::ALL`). */
const KATEGORIEN = ['system', 'browsers', 'apps', 'installers', 'recyclebin', 'registry'];

let haken = { analysieren: () => {}, bereinigen: () => {} };

/** Knöpfe und Werkzeugleiste einmalig verdrahten. */
export function verdrahte(neueHaken) {
    haken = { ...haken, ...neueHaken };

    $('scan-button')?.addEventListener('click', () => haken.analysieren());
    $('clean-button')?.addEventListener('click', () => haken.bereinigen());

    $('select-all')?.addEventListener('click', () => {
        setzeAuswahl(() => true);
        zeichne();
    });
    $('select-none')?.addEventListener('click', () => {
        zustand.auswahl.clear();
        zeichne();
    });
    $('select-recommended')?.addEventListener('click', () => {
        setzeAuswahl((ziel) => ziel.default_enabled);
        zeichne();
    });
    $('dry-run-toggle')?.addEventListener('change', (e) => {
        zustand.einstellungen = {
            ...zustand.einstellungen,
            dry_run_default: e.target.checked,
        };
        zeichne();
    });
    $('result-close')?.addEventListener('click', schliesseErgebnis);
    $('result-close-x')?.addEventListener('click', schliesseErgebnis);
}

/** Alles neu beschriften und zeichnen. */
export function zeichne() {
    beschrifte();
    zeichneListe();
    zeichneKopf();
}

// ---------------------------------------------------------------------------
// Kopfbereich
// ---------------------------------------------------------------------------

function beschrifte() {
    setzeText('home-title', t('home.title'));
    setzeText('home-subtitle', t('home.subtitle'));
    setzeText('select-all', t('home.select_all'));
    setzeText('select-none', t('home.select_none'));
    setzeText('select-recommended', t('home.select_recommended'));
    setzeText('dry-run-label', t('home.dry_run'));

    const trockenlauf = Boolean(zustand.einstellungen?.dry_run_default);
    const schalter = $('dry-run-toggle');
    if (schalter) schalter.checked = trockenlauf;
}

function zeichneKopf() {
    const laeuft = zustand.laeuft !== null;
    const bericht = zustand.bericht;

    const scanKnopf = $('scan-button');
    if (scanKnopf) {
        scanKnopf.textContent = t(zustand.laeuft === 'scan' ? 'home.analyzing' : 'home.analyze');
        scanKnopf.disabled = laeuft || zustand.laedt;
    }

    // Vor der ersten Analyse gibt es nichts zu bereinigen – der Knopf bleibt weg.
    const cleanKnopf = $('clean-button');
    if (cleanKnopf) {
        const groesse = ausgewaehlteGroesse();
        cleanKnopf.hidden = !bericht;
        cleanKnopf.disabled = laeuft || zustand.auswahl.size === 0;
        cleanKnopf.textContent =
            zustand.laeuft === 'clean'
                ? t('home.cleaning')
                : `${t('home.clean')} · ${t('home.selected_size', fmt.bytes(groesse))}`;
    }

    for (const id of ['select-all', 'select-none', 'select-recommended']) {
        const knopf = $(id);
        if (knopf) knopf.disabled = laeuft || !bericht;
    }

    if (!bericht) {
        setzeText('scan-headline', '');
        setzeText('scan-note', t('home.never_scanned'));
        return;
    }

    if (bericht.total_items === 0) {
        setzeText('scan-headline', t('home.nothing_found'));
        setzeText('scan-note', '');
        return;
    }

    setzeText(
        'scan-headline',
        t('home.found_total', fmt.bytes(bericht.total_size), fmt.zahl(bericht.total_items))
    );
    setzeText(
        'scan-note',
        `${t('home.selected_size', fmt.bytes(ausgewaehlteGroesse()))} · ` +
            `${fmt.zahl(ausgewaehlteAnzahl())}`
    );
}

// ---------------------------------------------------------------------------
// Zielliste
// ---------------------------------------------------------------------------

function zeichneListe() {
    const liste = $('target-list');
    if (!liste) return;

    // Solange der Katalog lädt, bleibt das Skelett stehen – eine leere weiße
    // Fläche würde wie ein Fehler aussehen.
    zeige($('target-skeleton'), zustand.laedt);
    if (zustand.laedt) {
        liste.replaceChildren();
        return;
    }

    const gruppen = KATEGORIEN.map(baueKategorie).filter(Boolean);
    liste.replaceChildren(...gruppen);
}

function baueKategorie(kategorie) {
    const ziele = zieleDerKategorie(kategorie);
    if (ziele.length === 0) return null;

    const offen = !zustand.zugeklappt.has(kategorie);
    const waehlbare = ziele.filter(istWaehlbar);
    const gewaehlte = waehlbare.filter((ziel) => zustand.auswahl.has(ziel.key));
    const groesse = ziele.reduce((summe, ziel) => summe + (scanVon(ziel.key)?.size ?? 0), 0);

    const sammelbox = el('input', {
        klasse: 'category-check',
        type: 'checkbox',
        attr: { 'aria-label': t(`category.${kategorie}`) },
    });
    sammelbox.disabled = waehlbare.length === 0;
    sammelbox.checked = waehlbare.length > 0 && gewaehlte.length === waehlbare.length;
    sammelbox.indeterminate = gewaehlte.length > 0 && gewaehlte.length < waehlbare.length;
    sammelbox.addEventListener('change', () => {
        const anhaken = sammelbox.checked;
        for (const ziel of waehlbare) {
            if (anhaken) {
                if (ziel.suggestion_only) waehleAllePfade(ziel.key);
                zustand.auswahl.add(ziel.key);
            } else {
                zustand.auswahl.delete(ziel.key);
            }
        }
        zeichne();
    });

    const umschalter = el('button', {
        klasse: 'category-toggle',
        type: 'button',
        attr: { 'aria-expanded': offen ? 'true' : 'false' },
    }, [
        el('span', { klasse: 'chevron', text: offen ? '▾' : '▸', attr: { 'aria-hidden': 'true' } }),
        el('span', { klasse: 'category-name', text: t(`category.${kategorie}`) }),
        el('span', {
            klasse: 'category-size',
            text: zustand.bericht ? fmt.bytes(groesse) : '',
        }),
    ]);
    umschalter.addEventListener('click', () => {
        if (offen) zustand.zugeklappt.add(kategorie);
        else zustand.zugeklappt.delete(kategorie);
        zeichne();
    });

    const koerper = el('div', { klasse: 'category-body' }, ziele.map(baueZiel));
    koerper.hidden = !offen;

    return el('section', { klasse: 'category', dataset: { kategorie } }, [
        el('div', { klasse: 'category-head' }, [sammelbox, umschalter]),
        koerper,
    ]);
}

function baueZiel(ziel) {
    const scan = scanVon(ziel.key);
    const waehlbar = istWaehlbar(ziel);
    const gewaehlt = zustand.auswahl.has(ziel.key);

    const box = el('input', { klasse: 'target-check', type: 'checkbox' });
    box.disabled = !waehlbar;
    box.checked = gewaehlt;
    box.addEventListener('change', () => {
        if (box.checked) {
            // Bei Vorschlagszielen ist „Ziel an“ ohne Datei sinnlos – deshalb
            // beim Anhaken alle gefundenen Dateien mitnehmen.
            if (ziel.suggestion_only) waehleAllePfade(ziel.key);
            zustand.auswahl.add(ziel.key);
        } else {
            zustand.auswahl.delete(ziel.key);
            if (ziel.suggestion_only) pfadeVon(ziel.key).clear();
        }
        zeichne();
    });

    const abzeichen = el('span', { klasse: 'badges' }, [
        el('span', {
            klasse: `badge risk risk-${ziel.risk}`,
            text: t(`risk.${ziel.risk}`),
            attr: { title: t(`risk.${ziel.risk}.hint`) },
        }),
        ziel.requires_admin
            ? el('span', { klasse: 'badge badge-admin', text: t('risk.needs_admin') })
            : null,
        ziel.suggestion_only
            ? el('span', { klasse: 'badge badge-suggestion', text: t('risk.suggestion_only') })
            : null,
    ]);

    const koerper = el('div', { klasse: 'target-body' }, [
        el('div', { klasse: 'target-head' }, [
            el('span', { klasse: 'target-name', text: t(ziel.name_key) }),
            abzeichen,
        ]),
        el('p', { klasse: 'target-desc', text: t(ziel.description_key) }),
        baueMeldungen(scan),
    ]);

    const zahlen = el('div', { klasse: 'target-figures' }, [
        el('span', {
            klasse: 'target-size',
            text: scan ? fmt.bytes(ziel.suggestion_only ? summePfade(ziel.key) : scan.size) : '',
        }),
        el('span', {
            klasse: 'target-count',
            text: scan ? fmt.zahl(scan.item_count) : t('home.never_scanned'),
        }),
    ]);

    const zeile = el('div', { klasse: 'target-row' }, [
        el('label', { klasse: 'target-main' }, [box, koerper]),
        zahlen,
    ]);

    if (ziel.suggestion_only && scan && scan.items.length > 0) {
        zeile.appendChild(baueDetailKnopf(ziel, scan));
    }

    const behaelter = el('div', {
        klasse: 'target' + (scan?.skipped ? ' is-skipped' : '') + (gewaehlt ? ' is-selected' : ''),
        dataset: { key: ziel.key },
    }, [zeile]);

    if (ziel.suggestion_only && scan && zustand.detailsOffen.has(ziel.key)) {
        behaelter.appendChild(baueDateiliste(ziel, scan));
    }
    return behaelter;
}

/** Übersprungen-Grund und Warnungen des Backends anzeigen. */
function baueMeldungen(scan) {
    if (!scan) return null;
    const meldungen = [];
    if (scan.skipped && scan.skip_reason) {
        meldungen.push(el('li', { klasse: 'meldung meldung-skip', text: tMeldung(scan.skip_reason) }));
    }
    for (const warnung of scan.warnings ?? []) {
        meldungen.push(el('li', { klasse: 'meldung meldung-warn', text: tMeldung(warnung) }));
    }
    if (meldungen.length === 0) return null;
    return el('ul', { klasse: 'meldungen' }, meldungen);
}

function baueDetailKnopf(ziel, scan) {
    const offen = zustand.detailsOffen.has(ziel.key);
    const knopf = el('button', {
        klasse: 'target-toggle',
        type: 'button',
        text: `${offen ? '▾' : '▸'} ${fmt.zahl(scan.items.length)}`,
        attr: {
            'aria-expanded': offen ? 'true' : 'false',
            'aria-label': t('risk.suggestion_only'),
        },
    });
    knopf.addEventListener('click', () => {
        if (offen) zustand.detailsOffen.delete(ziel.key);
        else zustand.detailsOffen.add(ziel.key);
        zeichne();
    });
    return knopf;
}

/**
 * Einzelliste eines Vorschlagsziels.
 *
 * Diese Pfade gehen als `only_paths` in den Bereinigungsauftrag – Plane löscht
 * hier grundsätzlich nur, was ausdrücklich angehakt wurde.
 */
function baueDateiliste(ziel, scan) {
    const gewaehlte = pfadeVon(ziel.key);

    const eintraege = scan.items.map((item) => {
        const box = el('input', { klasse: 'item-check', type: 'checkbox' });
        box.checked = gewaehlte.has(item.path);
        box.addEventListener('change', () => {
            if (box.checked) gewaehlte.add(item.path);
            else gewaehlte.delete(item.path);
            // Das Ziel selbst gilt genau dann als gewählt, wenn mindestens
            // eine Datei angehakt ist.
            if (gewaehlte.size > 0) zustand.auswahl.add(ziel.key);
            else zustand.auswahl.delete(ziel.key);
            zeichne();
        });

        // `detail` kommt als "installer|214" bzw. "archive|214" aus der Engine.
        // Angezeigt werden nur sprachneutrale Angaben: Endung und Alter.
        const alter = String(item.detail ?? '').split('|')[1];

        return el('li', { klasse: 'item-row' }, [
            el('label', { klasse: 'item-main' }, [
                box,
                el('span', { klasse: 'item-name', text: fmt.dateiname(item.path) }),
            ]),
            el('span', { klasse: 'item-path', text: fmt.kuerzePfad(item.path, 60), attr: { title: item.path } }),
            el('span', { klasse: 'item-meta', text: fmt.endung(item.path) }),
            el('span', { klasse: 'item-meta', text: alter ? `${fmt.zahl(alter)} d` : '' }),
            el('span', { klasse: 'item-size', text: fmt.bytes(item.size) }),
        ]);
    });

    return el('ul', { klasse: 'item-list' }, eintraege);
}

function waehleAllePfade(key) {
    const scan = scanVon(key);
    const gewaehlte = pfadeVon(key);
    for (const item of scan?.items ?? []) gewaehlte.add(item.path);
}
