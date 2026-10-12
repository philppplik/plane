/**
 * Ergebnis einer Bereinigung.
 *
 * Die Ansicht macht vier Dinge sichtbar, die sonst gern untergehen: was
 * wirklich freigegeben wurde, was übersprungen wurde und warum, ob der Lauf
 * überhaupt echt war oder nur simuliert – und **was aus welchem Grund
 * liegenblieb**.
 *
 * Der letzte Punkt ist der wichtigste. Auf einem laufenden Windows ist immer
 * irgendeine Cache-Datei geöffnet; dazu kommen Ordner, die Windows
 * grundsätzlich niemandem öffnet. Ohne diese Zeilen wirkt ein völlig normaler
 * Lauf lückenhaft, und wer nachfragt, bekommt keine Antwort.
 *
 * Der Bericht steht in einem Dialog, nicht in einem Abschnitt am Seitenende:
 * er ist der Moment, auf den der ganze Lauf hinausläuft. Wer dafür scrollen
 * muss, sieht ihn nicht.
 */

import { $, el, zeige, setzeText, fokusFalle } from './dom.js';
import { t, tMeldung } from './i18n.js';
import * as fmt from './format.js';

/** Zeichen je Zustand. Bewusst Text und kein Bild – skaliert mit der Schrift. */
const SYMBOL = { ok: '✓', skipped: '–', failed: '!' };

/**
 * Zustände, die kein Fehler sind: Zähler im Bericht → Text und Erklärung.
 *
 * Die Reihenfolge ist die der Häufigkeit, nicht die der Schwere.
 */
const HINWEISE = [
    ['total_locked', 'clean.locked', 'clean.locked_hint'],
    ['total_denied', 'clean.denied', 'clean.denied_hint'],
    ['total_blocked', 'clean.blocked', 'clean.blocked_hint'],
];

let falleAufheben = null;

/**
 * Bericht anzeigen.
 *
 * @param {object} bericht Zusammengeführter `CleanReport`.
 * @param {boolean} simulation `true`, wenn nur simuliert wurde.
 */
export function zeigeErgebnis(bericht, simulation) {
    const modal = $('result-modal');
    if (!modal || !bericht) return;

    setzeText('result-title', t('result.title'));
    setzeText('result-close', t('settings.close'));
    $('result-close-x')?.setAttribute('aria-label', t('settings.close'));

    const abzeichen = $('result-badge');
    abzeichen.textContent = bericht.cancelled
        ? t('status.cancelled')
        : simulation
          ? t('home.dry_run')
          : bericht.success
            ? t('status.ok')
            : t('status.failed');
    abzeichen.className = `badge ${zustandsKlasse(bericht, simulation)}`;

    setzeText(
        'result-total',
        t(
            simulation ? 'clean.dry_run_summary' : 'clean.summary',
            fmt.bytes(bericht.total_freed),
            fmt.zahl(bericht.total_removed)
        )
    );

    const sicherung = $('result-backup');
    zeige(sicherung, Boolean(bericht.registry_backup));
    if (bericht.registry_backup) {
        sicherung.textContent = t('clean.registry_backup', bericht.registry_backup);
        sicherung.title = bericht.registry_backup;
    }

    $('result-notes').replaceChildren(...baueHinweise(bericht));
    $('result-list').replaceChildren(...bericht.targets.map(baueZeile));

    if (bericht.error) {
        $('result-list').prepend(
            el('li', { klasse: 'result-item status-failed' }, [
                el('span', { klasse: 'result-name', text: tMeldung(bericht.error) }),
            ])
        );
    }

    zeige(modal, true);
    falleAufheben = fokusFalle(modal, schliesse);
}

export function schliesse() {
    zeige($('result-modal'), false);
    falleAufheben?.();
    falleAufheben = null;
}

/**
 * Zeilen für die erwartbaren Zustände.
 *
 * Jede nennt die Zahl **und** was man dagegen tun kann – auch dann, wenn die
 * Antwort „nichts, und das ist in Ordnung" lautet.
 */
function baueHinweise(bericht) {
    return HINWEISE.filter(([feld]) => (bericht[feld] ?? 0) > 0).map(
        ([feld, titel, erklaerung]) =>
            el('li', { klasse: 'result-note' }, [
                el('span', {
                    klasse: 'result-note-title',
                    text: t(titel, fmt.zahl(bericht[feld])),
                }),
                el('span', { klasse: 'result-note-hint', text: t(erklaerung) }),
            ])
    );
}

function zustandsKlasse(bericht, simulation) {
    if (bericht.cancelled) return 'badge-neutral';
    if (simulation) return 'badge-info';
    return bericht.success ? 'badge-ok' : 'badge-failed';
}

function baueZeile(ziel) {
    const zustand = ziel.skipped ? 'skipped' : ziel.ok ? 'ok' : 'failed';

    const fehler = (ziel.errors ?? []).map((meldung) =>
        el('li', { klasse: 'result-error', text: tMeldung(meldung) })
    );

    return el('li', { klasse: `result-item status-${zustand}` }, [
        el('span', { klasse: 'result-symbol', text: SYMBOL[zustand], attr: { 'aria-hidden': 'true' } }),
        el('span', { klasse: 'result-body' }, [
            el('span', { klasse: 'result-name', text: t(`target.${ziel.key}.name`) }),
            ziel.skipped && ziel.skip_reason
                ? el('span', { klasse: 'result-reason', text: tMeldung(ziel.skip_reason) })
                : null,
            fehler.length > 0 ? el('ul', { klasse: 'result-errors' }, fehler) : null,
        ]),
        el('span', {
            klasse: 'result-freed',
            text: ziel.skipped ? '' : fmt.bytes(ziel.freed),
        }),
    ]);
}
