/**
 * Über-Ansicht.
 *
 * Die Liste der Bibliotheken steht als statisches Markup im HTML: Namen und
 * Lizenzkürzel sind Eigennamen und werden nicht übersetzt. Übersetzt werden
 * nur die Überschriften und der Rechtestatus.
 */

import { $, el, setzeText } from './dom.js';
import { t } from './i18n.js';
import * as api from './api.js';
import * as fmt from './format.js';

/** Einmal geladene Markeninformation – ändert sich zur Laufzeit nicht. */
let marke = null;
let system = null;

/** Marke und Systeminformationen holen. */
export async function lade() {
    marke = marke ?? (await api.holeMarke());
    system = await api.holeSystem();
}

/** Ansicht mit den aktuellen Übersetzungen neu füllen. */
export function zeichne() {
    setzeText('about-subtitle', t('about.subtitle'));
    setzeText('about-made-in', t('about.made_in'));
    setzeText('about-tech-title', t('about.technical'));
    setzeText('about-opensource-title', t('about.opensource'));
    setzeText('about-system-title', t('about.system'));
    setzeText('about-back', t('nav.back'));

    if (marke) {
        setzeText('about-version', `v${marke.version} · ${marke.build_date}`);
        setzeText('about-copyright', `Plane © ${marke.build_date.slice(0, 4)} ${marke.developer}`);
    }

    const liste = $('about-system-list');
    if (!liste) return;

    if (!system) {
        liste.replaceChildren(el('li', { text: '…' }));
        return;
    }

    const gb = (wert) => `${fmt.zahl(wert, 1)} GB`;
    const zeilen = [
        system.os,
        system.cpu,
        `${system.arch} · ${gb(system.ram_gb)}`,
        `${system.disk.drive} · ${gb(system.disk.free_gb)} / ${gb(system.disk.total_gb)}`,
    ];
    liste.replaceChildren(...zeilen.map((zeile) => el('li', { text: zeile })));

    const rechte = $('about-admin');
    if (rechte) {
        rechte.textContent = t(system.is_admin ? 'about.admin_yes' : 'about.admin_no');
        rechte.classList.toggle('is-warning', !system.is_admin);
    }
}
