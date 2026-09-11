"""Vertragstests für die Oberfläche.

Statische Prüfungen über Markup, Stylesheets und JavaScript. Sie brauchen
weder einen Browser noch eine laufende App und fangen genau die Fehler ab, die
sonst erst im fertigen Build auffallen: ein Aufruf auf einen Command, den es
nicht gibt; eine Element-ID, die im Markup fehlt; ein deutscher Text, der
hartkodiert statt übersetzt ist.
"""

from __future__ import annotations

import re
from pathlib import Path
from typing import Set

import pytest

PROJEKT_WURZEL = Path(__file__).resolve().parents[1]
FRONTEND = PROJEKT_WURZEL / "frontend"
INDEX = FRONTEND / "index.html"
MAIN_JS = FRONTEND / "main.js"
SRC = FRONTEND / "src"
STYLES = FRONTEND / "styles"
LIB_RS = PROJEKT_WURZEL / "src-tauri" / "src" / "lib.rs"
I18N_RS = PROJEKT_WURZEL / "src-tauri" / "src" / "i18n.rs"


def _lies(pfad: Path) -> str:
    return pfad.read_text(encoding="utf-8")


def js_dateien() -> list[Path]:
    return [MAIN_JS, *sorted(SRC.glob("*.js"))]


def js_quelle() -> str:
    return "\n".join(_lies(p) for p in js_dateien())


def css_quelle() -> str:
    return "\n".join(_lies(p) for p in sorted(STYLES.glob("*.css")))


def registrierte_commands() -> Set[str]:
    block = re.search(r"generate_handler!\[(.*?)\]", _lies(LIB_RS), re.S)
    assert block, "generate_handler![...] nicht gefunden"
    return set(re.findall(r"commands::(\w+)", block.group(1)))


def i18n_schluessel() -> Set[str]:
    quelle = _lies(I18N_RS).split("#[cfg(test)]", 1)[0]
    return set(re.findall(r'\(\s*"([a-z0-9._]+)"\s*,\s*"', quelle))


# --------------------------------------------------------------------------
# Aufbau
# --------------------------------------------------------------------------


def test_es_gibt_genau_eine_html_datei():
    """Plane ist eine Single-Page-Anwendung; mehrere Seiten würden Markup
    duplizieren."""
    seiten = sorted(FRONTEND.glob("*.html"))
    assert seiten == [INDEX], f"Unerwartete Seiten: {[p.name for p in seiten]}"


def test_alle_bildschirme_liegen_im_markup():
    quelle = _lies(INDEX)
    for element_id in ("welcome-screen", "shell-screen", "home-screen", "about-screen"):
        assert f'id="{element_id}"' in quelle, f"Bildschirm fehlt: {element_id}"


def test_dokument_beginnt_mit_doctype_und_ist_deutsch_ausgezeichnet():
    quelle = _lies(INDEX).lstrip()
    assert quelle.lower().startswith("<!doctype html>")
    assert 'charset="UTF-8"' in quelle or 'charset="utf-8"' in quelle


def test_javascript_ist_in_module_aufgeteilt():
    """Eine einzelne Riesendatei wäre nicht wartbar."""
    module = list(SRC.glob("*.js"))
    assert len(module) >= 8, f"nur {len(module)} Module"
    assert len(_lies(MAIN_JS).splitlines()) < 300, "main.js sollte schlank bleiben"


def test_stylesheets_sind_eingebunden():
    quelle = _lies(INDEX)
    for datei in STYLES.glob("*.css"):
        assert datei.name in quelle, f"Stylesheet nicht eingebunden: {datei.name}"


# --------------------------------------------------------------------------
# Vertrag zum Backend
# --------------------------------------------------------------------------


def test_tauri_v2_importpfad():
    """In Tauri 2 liegt `invoke` unter '@tauri-apps/api/core'."""
    quelle = js_quelle()
    assert "@tauri-apps/api/core" in quelle
    assert "@tauri-apps/api/tauri" not in quelle, "Importpfad von Tauri 1.x"


def test_alle_aufgerufenen_commands_existieren():
    aufgerufen = set(re.findall(r"invoke\(\s*['\"]([\w_]+)['\"]", js_quelle()))
    fehlend = aufgerufen - registrierte_commands()
    assert not fehlend, f"Frontend ruft unbekannte Commands auf: {fehlend}"


def test_kernfunktionen_werden_aufgerufen():
    """Die namensgebende Funktion darf nicht nur im Backend existieren.

    ``api.js`` kapselt jeden Command in eine Hilfsfunktion; gesucht wird
    deshalb der Command-Name als Zeichenkette, nicht der ``invoke``-Aufruf.
    """
    quelle = _lies(SRC / "api.js")
    for command in ("scan", "clean", "cancel_run", "list_targets", "get_translations"):
        assert re.search(rf"['\"]{command}['\"]", quelle), f"{command} wird nie aufgerufen"


def test_invoke_laeuft_nur_ueber_das_api_modul():
    """Ein zentraler Zugang macht Fehler sichtbar und hält die Namen an einer
    Stelle."""
    for datei in js_dateien():
        if datei.name == "api.js":
            continue
        assert "invoke(" not in _lies(datei), (
            f"{datei.name} ruft invoke direkt auf – bitte über api.js"
        )


def test_fortschrittsereignis_wird_abonniert():
    assert "plane://progress" in js_quelle(), "Fortschritt wird nie empfangen"


def test_navigationsereignisse_werden_abonniert():
    quelle = js_quelle()
    for ereignis in ("navigate-home", "navigate-about", "navigate-welcome"):
        assert ereignis in quelle, f"Event nicht abonniert: {ereignis}"


# --------------------------------------------------------------------------
# Markup und Skript passen zusammen
# --------------------------------------------------------------------------


def test_referenzierte_element_ids_existieren():
    vorhandene = set(re.findall(r'id="([\w-]+)"', _lies(INDEX)))
    quelle = js_quelle()
    referenziert = set(re.findall(r"\$\(\s*['\"]([\w-]+)['\"]\s*\)", quelle))
    referenziert |= set(re.findall(r"setzeText\(\s*['\"]([\w-]+)['\"]", quelle))
    referenziert |= set(re.findall(r"getElementById\(\s*['\"]([\w-]+)['\"]", quelle))

    fehlend = referenziert - vorhandene
    assert not fehlend, f"Im JS referenziert, aber nicht im Markup: {sorted(fehlend)}"


def test_keine_inline_event_handler():
    """Inline-Handler brauchen globale Variablen und verhindern eine strikte
    Content-Security-Policy."""
    treffer = re.findall(r"\son[a-z]+\s*=\s*[\"']", _lies(INDEX))
    assert not treffer, f"Inline-Handler gefunden: {treffer}"


def test_kein_unverarbeitetes_template_markup():
    assert "<%" not in _lies(INDEX)


def ohne_kommentare(quelle: str) -> str:
    """Kommentare entfernen – sonst schlagen Prüfungen auf Erklärtexte an."""
    quelle = re.sub(r"/\*.*?\*/", "", quelle, flags=re.S)
    return re.sub(r"^\s*//.*$", "", quelle, flags=re.M)


def test_kein_innerhtml():
    """Fremde Zeichenketten dürfen nie als HTML interpretiert werden.

    Pfade und Fehlermeldungen stammen aus dem Dateisystem; sie als Markup zu
    behandeln wäre eine Einschleusungslücke.
    """
    for datei in js_dateien():
        code = ohne_kommentare(_lies(datei))
        assert "innerHTML" not in code, f"innerHTML in {datei.name}"


# --------------------------------------------------------------------------
# Sprache
# --------------------------------------------------------------------------


def test_alle_verwendeten_uebersetzungsschluessel_existieren():
    schluessel = i18n_schluessel()
    quelle = js_quelle()

    verwendet = set(re.findall(r"\bt\(\s*'([a-z][a-z0-9._]+)'", quelle))
    verwendet |= set(re.findall(r'data-i18n="([a-z][a-z0-9._]+)"', _lies(INDEX)))
    # Dynamisch zusammengesetzte Schlüssel (target.*, risk.*) lassen sich
    # statisch nicht auflösen und werden vom Backend-Vertragstest abgedeckt.
    verwendet = {k for k in verwendet if not k.startswith(("target.", "category."))}

    fehlend = verwendet - schluessel
    assert not fehlend, f"Unbekannte Übersetzungsschlüssel: {sorted(fehlend)}"


def test_keine_hartkodierten_oberflaechentexte():
    """Sichtbare Texte gehören in den Sprachkatalog, nicht ins Markup."""
    quelle = _lies(INDEX)

    # Erlaubt sind Eigennamen, Versionen, Lizenzkürzel und technische Angaben
    # wie Pfade – die werden bewusst nicht übersetzt.
    unbedenklich = re.compile(r"^[A-Za-z0-9 .,:;·/%©—–_\\()+-]+$")

    verdaechtig = []
    for treffer in re.findall(r">([^<>{}]{4,})<", quelle):
        text = treffer.strip()
        if not text or text.startswith("&"):
            continue
        if unbedenklich.match(text):
            continue
        verdaechtig.append(text)

    assert not verdaechtig, f"Hartkodierte Texte im Markup: {verdaechtig}"


def test_echte_umlaute_in_sichtbaren_texten():
    """Sichtbarer Text muss echte Umlaute tragen.

    Bezeichner im Code (``zeigeUebersicht``) sind davon ausgenommen: dort ist
    ASCII bewusst gewählt, damit Funktionsnamen überall tippbar bleiben.
    """
    markup = re.sub(r"<[^>]+>", " ", _lies(INDEX))
    for falsch in ("Uebersicht", "Loeschen", "Groesse", "Zurueck", "Auswaehlen"):
        assert falsch not in markup, f"Umlautersatz im Markup: '{falsch}'"

    # In Kommentaren und Texten müssen tatsächlich Umlaute vorkommen.
    assert any(c in js_quelle() for c in "äöüßÄÖÜ")
    assert any(c in _lies(INDEX) for c in "äöüßÄÖÜ")


# --------------------------------------------------------------------------
# Gestaltung
# --------------------------------------------------------------------------


def test_mehrere_breakpoints_vorhanden():
    """Das Fenster ist ab 620×560 skalierbar – ohne Breakpoints bricht das
    Layout."""
    treffer = re.findall(r"@media[^{]*max-width", css_quelle())
    assert len(treffer) >= 2, f"nur {len(treffer)} Breakpoints"


def test_dunkelmodus_wird_unterstuetzt():
    quelle = css_quelle()
    assert "prefers-color-scheme" in quelle
    assert '[data-theme="dark"]' in quelle


def test_bewegungsarme_darstellung_wird_respektiert():
    assert "prefers-reduced-motion" in css_quelle()


def test_farben_laufen_ueber_variablen():
    """Ohne Variablen wäre der Dunkelmodus nicht pflegbar."""
    quelle = css_quelle()
    assert quelle.count("var(--") > 80


def test_fokus_ist_sichtbar():
    assert ":focus-visible" in css_quelle(), "Tastaturbedienung braucht einen Fokusring"


@pytest.mark.parametrize(
    "klasse",
    ["target-list", "overlay", "progress-bar-large", "toast", "badge", "switch"],
)
def test_tragende_bausteine_sind_gestaltet(klasse: str):
    assert f".{klasse}" in css_quelle(), f"Stil fehlt: .{klasse}"
