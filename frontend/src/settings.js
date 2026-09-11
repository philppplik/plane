/**
 * Einstellungsdialog.
 *
 * Es gibt keinen „Speichern“-Knopf: jede Änderung geht sofort per
 * `set_settings` ins Backend. Das Backend ist damit die einzige Wahrheit,
 * und ein Absturz kann keine halb übernommenen Einstellungen hinterlassen.
 */

import { $, el, zeige, fokusFalle } from './dom.js';
import { t } from './i18n.js';
import { zustand } from './state.js';
import * as api from './api.js';
import { setzeTheme } from './theme.js';
import * as update from './update.js';

/** Wird gesetzt, sobald der Dialog verdrahtet ist. */
let beiSprachwechsel = async () => {};
let beiAenderung = () => {};
let falleAufheben = null;

/**
 * Dialog einmalig verdrahten.
 *
 * @param {object} haken Rückrufe der Anwendung: `sprachwechsel` lädt den
 *   Katalog neu und zeichnet alles neu, `aenderung` gleicht abhängige
 *   Bedienelemente (z. B. den Simulationsschalter) ab.
 */
export function verdrahte(haken) {
    beiSprachwechsel = haken.sprachwechsel ?? beiSprachwechsel;
    beiAenderung = haken.aenderung ?? beiAenderung;

    $('nav-settings')?.addEventListener('click', oeffne);
    $('settings-close')?.addEventListener('click', schliesse);
    $('settings-close-x')?.addEventListener('click', schliesse);

    $('settings-language')?.addEventListener('change', async (e) => {
        await speichere({ language: e.target.value });
        await beiSprachwechsel();
        beschrifte();
    });

    for (const id of ['theme-system', 'theme-light', 'theme-dark']) {
        $(id)?.addEventListener('click', async (e) => {
            const theme = e.currentTarget.dataset.theme;
            setzeTheme(theme);
            await speichere({ theme });
            markiereTheme(theme);
        });
    }

    $('settings-confirm-risky')?.addEventListener('change', (e) =>
        speichere({ confirm_risky: e.target.checked })
    );

    $('settings-dry-run')?.addEventListener('change', async (e) => {
        await speichere({ dry_run_default: e.target.checked });
        beiAenderung();
    });

    // Der einzige Schalter, der Netzwerkverkehr auslöst. Er steht bewusst
    // ganz unten und mit ausführlichem Hinweistext daneben.
    $('settings-check-updates')?.addEventListener('change', (e) =>
        speichere({ check_updates: e.target.checked })
    );

    $('settings-check-now')?.addEventListener('click', () => update.pruefeJetzt());

    $('settings-reset-welcome')?.addEventListener('click', async () => {
        await api.setzeWillkommenZurueck();
        melde($('settings-reset-welcome'));
    });
}

/** Beschriftungen neu setzen – nach jedem Sprachwechsel nötig. */
export function beschrifte() {
    $('settings-title').textContent = t('settings.title');
    $('settings-language-label').textContent = t('settings.language');
    $('settings-language-hint').textContent = t('settings.language.hint');
    $('settings-theme-label').textContent = t('settings.theme');
    $('theme-system').textContent = t('settings.theme.system');
    $('theme-light').textContent = t('settings.theme.light');
    $('theme-dark').textContent = t('settings.theme.dark');
    $('settings-confirm-risky-label').textContent = t('settings.confirm_risky');
    $('settings-confirm-risky-hint').textContent = t('settings.confirm_risky.hint');
    $('settings-dry-run-label').textContent = t('settings.dry_run_default');
    $('settings-dry-run-hint').textContent = t('settings.dry_run_default.hint');
    update.beschrifte();
    $('settings-reset-welcome').textContent = t('settings.reset_welcome');
    $('settings-close').textContent = t('settings.close');
    $('settings-close-x').setAttribute('aria-label', t('settings.close'));
}

/** Bedienelemente an den Zustand angleichen. */
export function spiegele() {
    const einstellungen = zustand.einstellungen;
    if (!einstellungen) return;

    const auswahl = $('settings-language');
    auswahl.replaceChildren(
        ...zustand.sprachen.map((sprache) =>
            el('option', { text: sprache.label, value: sprache.code })
        )
    );
    auswahl.value = einstellungen.language;

    markiereTheme(einstellungen.theme);
    $('settings-confirm-risky').checked = Boolean(einstellungen.confirm_risky);
    $('settings-dry-run').checked = Boolean(einstellungen.dry_run_default);
    $('settings-check-updates').checked = Boolean(einstellungen.check_updates);
}

function markiereTheme(theme) {
    for (const id of ['theme-system', 'theme-light', 'theme-dark']) {
        const knopf = $(id);
        const aktiv = knopf.dataset.theme === theme;
        knopf.classList.toggle('is-active', aktiv);
        knopf.setAttribute('aria-checked', aktiv ? 'true' : 'false');
    }
}

/** Teiländerung übernehmen und persistieren. */
async function speichere(teil) {
    zustand.einstellungen = { ...zustand.einstellungen, ...teil };
    const gespeichert = await api.speichereEinstellungen(zustand.einstellungen);
    if (gespeichert) zustand.einstellungen = gespeichert;
    melde($('settings-title'));
}

/** Kurze Rückmeldung, dass etwas angekommen ist. */
function melde(bezug) {
    if (!bezug) return;
    bezug.classList.add('saved-flash');
    setTimeout(() => bezug.classList.remove('saved-flash'), 600);
}

export function oeffne() {
    beschrifte();
    spiegele();
    const modal = $('settings-modal');
    zeige(modal, true);
    falleAufheben = fokusFalle(modal, schliesse);
}

export function schliesse() {
    zeige($('settings-modal'), false);
    falleAufheben?.();
    falleAufheben = null;
}
