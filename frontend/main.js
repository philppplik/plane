/**
 * Plane – Einstiegspunkt der Oberfläche.
 *
 * Diese Datei hält bewusst **keine** Fachlogik. Sie ist die
 * Kompositionswurzel: sie lädt den Sprachkatalog, verdrahtet die Module
 * miteinander und schaltet zwischen den Bildschirmen um. Alles andere liegt
 * in `src/` – Analyse und Bereinigung in `run.js`, die Zielliste in
 * `dashboard.js`, der Fortschritt in `progress.js`.
 *
 * Reihenfolge beim Start ist wichtig: erst Einstellungen und Sprache, dann
 * Beschriftungen, dann die Ziele. Andernfalls blitzen unübersetzte
 * Schlüssel auf.
 */

import { listen } from '@tauri-apps/api/event';

import * as api from './src/api.js';
import * as about from './src/about.js';
import * as dashboard from './src/dashboard.js';
import * as progress from './src/progress.js';
import * as programs from './src/programs.js';
import * as run from './src/run.js';
import * as settings from './src/settings.js';
import * as tweaks from './src/tweaks.js';
import * as fmt from './src/format.js';
import { $, zeige } from './src/dom.js';
import { setzeKatalog, t, uebersetzeMarkup } from './src/i18n.js';
import { zustand } from './src/state.js';
import { setzeTheme } from './src/theme.js';

/** Eventname, unter dem das Backend Fortschritt meldet. */
const FORTSCHRITT_EVENT = 'plane://progress';

/** Wie lange eine Fehlermeldung stehen bleibt. */
const TOAST_DAUER_MS = 6000;

/** Bildschirm-Kennung → Element-ID. */
const BILDSCHIRME = {
    welcome: 'welcome-screen',
    home: 'home-screen',
    programs: 'programs-screen',
    tweaks: 'tweaks-screen',
    about: 'about-screen',
};

/** Navigationsknopf → Bildschirm. */
const NAVIGATION = [
    ['nav-home', 'home'],
    ['nav-programs', 'programs'],
    ['nav-tweaks', 'tweaks'],
    ['nav-about', 'about'],
];

let toastZeitgeber = null;

// ---------------------------------------------------------------------------
// Meldungen
// ---------------------------------------------------------------------------

/**
 * Fehler sichtbar anzeigen.
 *
 * Ein fehlgeschlagener Backend-Aufruf darf nicht stumm in der Konsole landen –
 * für den Nutzer sähe das aus, als passiere einfach nichts.
 */
function zeigeMeldung(nachricht, istFehler = true) {
    const toast = $('toast');
    if (!toast) return;

    toast.textContent = nachricht;
    toast.classList.remove('is-hidden');
    toast.classList.toggle('is-error', istFehler);

    clearTimeout(toastZeitgeber);
    toastZeitgeber = setTimeout(() => toast.classList.add('is-hidden'), TOAST_DAUER_MS);
}

/** Kurzform für den häufigsten Fall. */
const zeigeFehler = (nachricht) => zeigeMeldung(nachricht, true);

// ---------------------------------------------------------------------------
// Bildschirme
// ---------------------------------------------------------------------------

/**
 * Bildschirm umschalten.
 *
 * Willkommen ist ein eigener Vollbild-Schirm; Übersicht und Über sind
 * Ansichten innerhalb des Anwendungsrahmens, damit die Seitenleiste beim
 * Wechsel stehen bleibt.
 */
function zeigeBildschirm(name) {
    const ziel = BILDSCHIRME[name] ? name : 'home';
    const istWillkommen = ziel === 'welcome';

    zeige($('welcome-screen'), istWillkommen);
    zeige($('shell-screen'), !istWillkommen);
    zeige($('home-screen'), ziel === 'home');
    zeige($('programs-screen'), ziel === 'programs');
    zeige($('tweaks-screen'), ziel === 'tweaks');
    zeige($('about-screen'), ziel === 'about');

    for (const [id, schirm] of NAVIGATION) {
        const knopf = $(id);
        if (!knopf) continue;
        knopf.classList.toggle('is-active', schirm === ziel);
        if (schirm === ziel) {
            knopf.setAttribute('aria-current', 'page');
        } else {
            knopf.removeAttribute('aria-current');
        }
    }

    // Programme und Windows-Einstellungen werden erst beim ersten Öffnen
    // geladen: beides fragt die Registry ab und kostet spürbar Zeit, die
    // beim Start niemand verschenken möchte.
    if (ziel === 'about') {
        about.lade().then(about.zeichne);
    } else if (ziel === 'programs') {
        programs.lade().catch((fehler) => zeigeFehler(`${fehler}`));
    } else if (ziel === 'tweaks') {
        tweaks.lade().catch((fehler) => zeigeFehler(`${fehler}`));
    }
}

// ---------------------------------------------------------------------------
// Sprache
// ---------------------------------------------------------------------------

/**
 * Katalog laden und die gesamte Oberfläche neu beschriften.
 *
 * Wird beim Start und nach jedem Sprachwechsel aufgerufen. Die Ziele selbst
 * bleiben geladen – sie tragen Übersetzungsschlüssel, keine Texte.
 */
async function ladeSprache(kuerzel) {
    const katalog = await api.holeUebersetzungen(kuerzel);
    setzeKatalog(katalog, kuerzel);

    document.documentElement.lang = kuerzel;
    uebersetzeMarkup();
    beschrifteRahmen();
    dashboard.zeichne();
    settings.beschrifte();
    about.zeichne();
    programs.beschrifte();
    tweaks.beschrifte();
}

/** Texte, die außerhalb der Module liegen (Willkommen, Seitenleiste). */
function beschrifteRahmen() {
    const setze = (id, text) => {
        const knoten = $(id);
        if (knoten) knoten.textContent = text;
    };

    setze('welcome-title', t('app.welcome_title'));
    setze('welcome-tagline', t('app.tagline'));
    setze('welcome-start', t('app.start'));
    setze('home-title', t('home.title'));
    setze('home-subtitle', t('home.subtitle'));
    setze('about-title', 'Plane');
}

// ---------------------------------------------------------------------------
// Laufwerksanzeige
// ---------------------------------------------------------------------------

/** Belegung des Systemlaufwerks in der Seitenleiste auffrischen. */
async function aktualisiereLaufwerk() {
    const stats = await api.holeLaufwerk();
    if (!stats) return;

    const setze = (id, text) => {
        const knoten = $(id);
        if (knoten) knoten.textContent = text;
    };

    setze('disk-title', t('disk.title', stats.drive));
    setze('disk-free', t('disk.free', `${fmt.zahl(stats.free_gb, 1)} GB`));
    setze('disk-total', t('disk.of_total', `${fmt.zahl(stats.total_gb, 1)} GB`));
    setze('disk-percent', t('disk.used_percent', fmt.zahl(stats.used_percent, 0)));

    const fuellung = $('disk-fill');
    if (fuellung) fuellung.style.width = `${fmt.prozent(stats.used_percent)}%`;
    $('disk-bar')?.setAttribute('aria-valuenow', String(Math.round(stats.used_percent)));
}

// ---------------------------------------------------------------------------
// Start
// ---------------------------------------------------------------------------

async function starte() {
    api.setzeFehlerAnzeige(zeigeFehler);

    // 1. Einstellungen zuerst – Sprache und Erscheinungsbild hängen daran.
    const [einstellungen, sprachen] = await Promise.all([
        api.holeEinstellungen(),
        api.holeSprachen(),
    ]);
    zustand.einstellungen = einstellungen ?? {
        language: 'en',
        theme: 'system',
        confirm_risky: true,
        dry_run_default: false,
        selected_targets: [],
    };
    zustand.sprachen = sprachen;

    setzeTheme(zustand.einstellungen.theme);
    await ladeSprache(zustand.einstellungen.language);

    // 2. Module verdrahten.
    dashboard.verdrahte({
        analysieren: run.analysiere,
        bereinigen: run.bereinige,
    });
    run.verdrahte({ laufwerk: aktualisiereLaufwerk });
    progress.verdrahte();
    settings.verdrahte({
        sprachwechsel: () => ladeSprache(zustand.einstellungen.language),
        aenderung: () => dashboard.zeichne(),
    });
    settings.spiegele();
    programs.verdrahte({ meldung: zeigeMeldung });
    tweaks.verdrahte({ meldung: zeigeMeldung });

    $('welcome-start')?.addEventListener('click', async () => {
        zeigeBildschirm(await api.starteApp());
    });
    $('nav-home')?.addEventListener('click', async () => {
        zeigeBildschirm(await api.zeigeUebersicht());
    });
    $('nav-programs')?.addEventListener('click', () => zeigeBildschirm('programs'));
    $('nav-tweaks')?.addEventListener('click', () => zeigeBildschirm('tweaks'));
    $('nav-about')?.addEventListener('click', async () => {
        zeigeBildschirm(await api.zeigeUeber());
    });
    $('about-back')?.addEventListener('click', async () => {
        zeigeBildschirm(await api.zeigeUebersicht());
    });

    // 3. Navigationsereignisse des Backends beachten – sonst wäre der
    //    Zustand im Backend führend, die Anzeige aber nicht.
    await listen('navigate-welcome', () => zeigeBildschirm('welcome'));
    await listen('navigate-home', () => zeigeBildschirm('home'));
    await listen('navigate-about', () => zeigeBildschirm('about'));

    // 4. Fortschritt: die einzige Stelle, die das Backend-Event abonniert.
    await listen(FORTSCHRITT_EVENT, (ereignis) => progress.aktualisiere(ereignis.payload));

    // 5. Inhalte laden.
    zustand.ziele = await api.holeZiele();
    zustand.laedt = false;
    dashboard.zeichne();

    zeigeBildschirm(await api.holeStartbildschirm());
    await aktualisiereLaufwerk();
}

document.addEventListener('DOMContentLoaded', () => {
    starte().catch((fehler) => zeigeFehler(`Start: ${fehler}`));
});
