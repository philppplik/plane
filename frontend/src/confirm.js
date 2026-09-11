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

    return frageNach({
        titel: t('confirm.title'),
        einleitung: t('confirm.intro'),
        punkte,
        warnung: t('confirm.warning'),
        zustimmen: t('confirm.accept'),
    });
}

/**
 * Denselben Dialog für eine einzelne Rückfrage nutzen.
 *
 * Deinstallation und Windows-Einstellungen brauchen dieselbe Fokusfalle und
 * dieselbe Tastaturbedienung wie die Bereinigung – ein zweiter Dialog wäre
 * eine zweite Fehlerquelle.
 *
 * @param {{titel: string, einleitung: string, punkte?: Array,
 *          warnung?: string, zustimmen: string}} vorgabe
 * @returns {Promise<boolean>}
 */
export function frageNach({ titel, einleitung, punkte = [], warnung = '', zustimmen }) {
    return new Promise((resolve) => {
        const modal = $('confirm-modal');

        $('confirm-title').textContent = titel;
        $('confirm-intro').textContent = einleitung;
        $('confirm-warning').textContent = warnung;
        zeige($('confirm-warning'), Boolean(warnung));
        $('confirm-accept').textContent = zustimmen;
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
