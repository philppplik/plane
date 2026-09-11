/**
 * Ergebnis einer Bereinigung.
 *
 * Die Ansicht macht drei Dinge sichtbar, die sonst gern untergehen: was
 * wirklich freigegeben wurde, was übersprungen wurde und warum – und ob der
 * Lauf überhaupt echt war oder nur simuliert.
 */

import { $, el, zeige, setzeText } from './dom.js';
import { t, tMeldung } from './i18n.js';
import * as fmt from './format.js';

/** Zeichen je Zustand. Bewusst Text und kein Bild – skaliert mit der Schrift. */
const SYMBOL = { ok: '✓', skipped: '–', failed: '!' };

/**
 * Bericht anzeigen.
 *
 * @param {object} bericht Zusammengeführter `CleanReport`.
 * @param {boolean} simulation `true`, wenn nur simuliert wurde.
 */
export function zeigeErgebnis(bericht, simulation) {
    const panel = $('result-panel');
    if (!panel || !bericht) return;

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

    setzeText('result-close', t('settings.close'));

    $('result-list').replaceChildren(...bericht.targets.map(baueZeile));

    if (bericht.error) {
        $('result-list').prepend(
            el('li', { klasse: 'result-item status-failed' }, [
                el('span', { klasse: 'result-name', text: tMeldung(bericht.error) }),
            ])
        );
    }

    zeige(panel, true);
    panel.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
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
