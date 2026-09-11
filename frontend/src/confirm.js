/**
 * Bestätigungsdialog vor riskanten Zielen.
 *
 * Plane löscht auch Unwiderrufliches (Papierkorb, Windows.old, Registry).
 * Deshalb gibt es hier keine pauschale Ja/Nein-Frage, sondern eine Liste:
 * jedes betroffene Ziel mit Namen **und** Grund. Wer zustimmt, soll wissen,
 * wofür.
 */

import { $, el, zeige, fokusFalle } from './dom.js';
import { t } from './i18n.js';

/**
 * Freigabe einholen.
 *
 * @param {Array<{name: string, grund: string}>} punkte Betroffene Ziele.
 * @returns {Promise<boolean>} `true`, wenn ausdrücklich zugestimmt wurde.
 */
export function freigabeEinholen(punkte) {
    if (!punkte || punkte.length === 0) return Promise.resolve(true);

    return new Promise((resolve) => {
        const modal = $('confirm-modal');

        $('confirm-title').textContent = t('confirm.title');
        $('confirm-intro').textContent = t('confirm.intro');
        $('confirm-warning').textContent = t('confirm.warning');
        $('confirm-accept').textContent = t('confirm.accept');
        $('confirm-cancel').textContent = t('confirm.cancel');
        $('confirm-close').setAttribute('aria-label', t('confirm.cancel'));

        const liste = $('confirm-list');
        liste.replaceChildren(
            ...punkte.map((punkt) =>
                el('li', { klasse: 'confirm-item' }, [
                    el('span', { klasse: 'confirm-item-name', text: punkt.name }),
                    el('span', { klasse: 'confirm-item-reason', text: punkt.grund }),
                ])
            )
        );

        zeige(modal, true);
        const falleAufheben = fokusFalle(modal, () => schliessen(false));

        function schliessen(antwort) {
            zeige(modal, false);
            $('confirm-accept').removeEventListener('click', ja);
            $('confirm-cancel').removeEventListener('click', nein);
            $('confirm-close').removeEventListener('click', nein);
            falleAufheben();
            resolve(antwort);
        }
        const ja = () => schliessen(true);
        const nein = () => schliessen(false);

        $('confirm-accept').addEventListener('click', ja);
        $('confirm-cancel').addEventListener('click', nein);
        $('confirm-close').addEventListener('click', nein);
        // Der harmlose Knopf bekommt den Fokus, nicht der gefährliche.
        $('confirm-cancel').focus();
    });
}
