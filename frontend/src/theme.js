/**
 * Erscheinungsbild.
 *
 * Bei „Wie das System“ wird bewusst *kein* Attribut gesetzt: dann greift die
 * `prefers-color-scheme`-Regel im Stylesheet und folgt Änderungen der
 * Windows-Einstellung sofort, ohne dass hier ein Listener nötig wäre.
 */

/** Theme anwenden: `"system"`, `"light"` oder `"dark"`. */
export function setzeTheme(theme) {
    const wurzel = document.documentElement;
    if (theme === 'light' || theme === 'dark') {
        wurzel.dataset.theme = theme;
    } else {
        delete wurzel.dataset.theme;
    }
    aktualisiereBrowserleiste();
}

/**
 * Die `theme-color`-Angabe an die tatsächliche Hintergrundfarbe angleichen,
 * damit der Fensterrahmen im dunklen Modus nicht hell aufblitzt.
 */
function aktualisiereBrowserleiste() {
    const meta = document.querySelector('meta[name="theme-color"]');
    if (!meta) return;
    const farbe = getComputedStyle(document.documentElement)
        .getPropertyValue('--bg')
        .trim();
    if (farbe) meta.setAttribute('content', farbe);
}
