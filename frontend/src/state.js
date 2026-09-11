/**
 * Gemeinsamer Zustand der Oberfläche.
 *
 * Ein einziges Objekt statt verteilter Modulvariablen: die Zielliste, die
 * Auswahl und das Analyseergebnis hängen so eng zusammen, dass getrennte
 * Ablagen unweigerlich auseinanderlaufen würden.
 */

export const zustand = {
    /** Alle Ziele des Katalogs (`list_targets`). */
    ziele: [],
    /** Analyseergebnis je Zielschlüssel (`TargetScan`). */
    scans: new Map(),
    /** Ausgewählte Zielschlüssel. */
    auswahl: new Set(),
    /** Einzeln angehakte Pfade je Ziel – nur für `suggestion_only`. */
    pfadAuswahl: new Map(),
    /** Zugeklappte Kategorien (Standard: alles offen). */
    zugeklappt: new Set(),
    /** Ziele mit ausgeklappter Einzelliste. */
    detailsOffen: new Set(),
    /** Aktuelle Einstellungen des Backends. */
    einstellungen: null,
    /** Verfügbare Sprachen. */
    sprachen: [],
    /** Letzter Analysebericht. */
    bericht: null,
    /** `'scan'`, `'clean'` oder `null`. */
    laeuft: null,
    /** Ziele werden gerade geladen – dann Skelett statt leerer Fläche. */
    laedt: true,
};

/** Analyseergebnis eines Ziels, falls vorhanden. */
export const scanVon = (key) => zustand.scans.get(key) ?? null;

/**
 * Ist das Ziel auswählbar?
 *
 * Übersprungene Ziele und solche ohne Fund sind nur Information – sie in die
 * Auswahl zu lassen, würde eine Bereinigung versprechen, die nichts tut.
 */
export function istWaehlbar(ziel) {
    const scan = scanVon(ziel.key);
    if (!scan) return false;
    return !scan.skipped && scan.item_count > 0;
}

/** Größe, die aktuell zur Bereinigung ansteht. */
export function ausgewaehlteGroesse() {
    let summe = 0;
    for (const key of zustand.auswahl) {
        const scan = scanVon(key);
        if (!scan) continue;
        summe += scan.suggestion_only ? summePfade(key) : scan.size;
    }
    return summe;
}

/** Anzahl der Einträge, die aktuell zur Bereinigung anstehen. */
export function ausgewaehlteAnzahl() {
    let summe = 0;
    for (const key of zustand.auswahl) {
        const scan = scanVon(key);
        if (!scan) continue;
        summe += scan.suggestion_only
            ? (zustand.pfadAuswahl.get(key)?.size ?? 0)
            : scan.item_count;
    }
    return summe;
}

/** Summe der einzeln angehakten Pfade eines Vorschlagsziels. */
export function summePfade(key) {
    const scan = scanVon(key);
    const gewaehlt = zustand.pfadAuswahl.get(key);
    if (!scan || !gewaehlt) return 0;
    return scan.items
        .filter((item) => gewaehlt.has(item.path))
        .reduce((summe, item) => summe + item.size, 0);
}

/** Pfadauswahl eines Ziels, bei Bedarf angelegt. */
export function pfadeVon(key) {
    if (!zustand.pfadAuswahl.has(key)) zustand.pfadAuswahl.set(key, new Set());
    return zustand.pfadAuswahl.get(key);
}

/** Auswahl auf einen Filter zurückführen (alles / nichts / empfohlen). */
export function setzeAuswahl(filter) {
    zustand.auswahl.clear();
    for (const ziel of zustand.ziele) {
        if (!istWaehlbar(ziel)) continue;
        if (filter(ziel, scanVon(ziel.key))) zustand.auswahl.add(ziel.key);
    }
    // Vorschlagsziele ohne angehakte Datei bringen nichts – wieder abwählen.
    for (const key of [...zustand.auswahl]) {
        const scan = scanVon(key);
        if (scan?.suggestion_only && (zustand.pfadAuswahl.get(key)?.size ?? 0) === 0) {
            zustand.auswahl.delete(key);
        }
    }
}

/** Ziele einer Kategorie in Katalogreihenfolge. */
export const zieleDerKategorie = (kategorie) =>
    zustand.ziele.filter((ziel) => ziel.category === kategorie);
