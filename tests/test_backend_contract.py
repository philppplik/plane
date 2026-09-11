"""Vertragstests für Backend, Konfiguration und Projekthygiene.

Diese Tests analysieren die Quelldateien statisch. Sie brauchen weder eine
laufende App noch die Rust-Toolchain und decken genau die Bruchstellen ab, die
sonst erst im fertigen MSI-Build auffallen.

Die Frontend-Seite prüft ``test_frontend_contract.py``, die Engine-Invarianten
prüfen die Rust-Unit-Tests in ``src-tauri/src/engine/``.
"""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Set

import pytest

PROJEKT_WURZEL = Path(__file__).resolve().parents[1]
SRC_TAURI = PROJEKT_WURZEL / "src-tauri"
SRC = SRC_TAURI / "src"
LIB_RS = SRC / "lib.rs"
COMMANDS_RS = SRC / "commands.rs"
CATALOG_RS = SRC / "engine" / "catalog.rs"
I18N_RS = SRC / "i18n.rs"


def _lies(pfad: Path) -> str:
    return pfad.read_text(encoding="utf-8")


def registrierte_commands() -> Set[str]:
    """Alle in ``generate_handler!`` registrierten Command-Namen."""
    block = re.search(r"generate_handler!\[(.*?)\]", _lies(LIB_RS), re.S)
    assert block, "generate_handler![...] nicht in lib.rs gefunden."
    return set(re.findall(r"commands::(\w+)", block.group(1)))


def definierte_commands() -> Set[str]:
    """Alle mit ``#[tauri::command]`` annotierten Funktionen."""
    return set(
        re.findall(
            r"#\[tauri::command\]\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)",
            _lies(COMMANDS_RS),
        )
    )


def katalog_schluessel() -> Set[str]:
    """Zielschlüssel aus dem Rust-Katalog (ohne das Testmodul darunter).

    Ziele werden auf zwei Arten deklariert: über den Kurzschreibweise-Helfer
    ``files("key", …)`` und als ausgeschriebenes ``Target { key: "key", … }``.
    """
    quelle = _lies(CATALOG_RS).split("pub fn target_by_key", 1)[0]
    schluessel = set(re.findall(r'key:\s*"([a-z0-9._]+)"', quelle))
    schluessel |= set(re.findall(r'files\(\s*"([a-z0-9._]+)"', quelle))
    return schluessel


def i18n_schluessel() -> Set[str]:
    """Alle Übersetzungsschlüssel."""
    quelle = _lies(I18N_RS).split("#[cfg(test)]", 1)[0]
    return set(re.findall(r'\(\s*"([a-z0-9._]+)"\s*,\s*"', quelle))


# --------------------------------------------------------------------------
# Command-Vertrag
# --------------------------------------------------------------------------


def test_jeder_definierte_command_ist_registriert():
    fehlend = definierte_commands() - registrierte_commands()
    assert not fehlend, f"Definiert, aber nicht in lib.rs registriert: {fehlend}"


def test_jeder_registrierte_command_existiert():
    fehlend = registrierte_commands() - definierte_commands()
    assert not fehlend, f"Registriert, aber nicht definiert: {fehlend}"


def test_kernfunktionen_sind_als_command_verfuegbar():
    """Ohne diese Commands ist die Anwendung funktionslos."""
    vorhanden = definierte_commands()
    for command in (
        "scan",
        "clean",
        "cancel_run",
        "list_targets",
        "get_translations",
        "get_settings",
        "set_settings",
        "get_disk_stats",
    ):
        assert command in vorhanden, f"Command fehlt: {command}"


def test_commands_liefern_typisierte_ergebnisse():
    """Handgebautes JSON als String entzieht sich der Compilerprüfung."""
    produktiv = _lies(COMMANDS_RS).split("#[cfg(test)]", 1)[0]
    assert 'r#"{' not in produktiv


def test_zustand_wird_ohne_unwrap_gesperrt():
    """Ein vergifteter Mutex würde sonst jeden weiteren Aufruf abstürzen
    lassen."""
    assert ".lock().unwrap()" not in _lies(COMMANDS_RS)


def test_module_werden_genau_einmal_kompiliert():
    """``main.rs`` darf die Module nicht zusätzlich deklarieren – sonst laufen
    alle Unit-Tests doppelt."""
    main_rs = _lies(SRC / "main.rs")
    assert "mod commands" not in main_rs
    assert "plane_lib::run()" in main_rs


def test_engine_kennt_die_oberflaeche_nicht():
    """Die Engine muss ohne Tauri lauffähig bleiben – nur so können CLI, TUI
    und Tests denselben Code verwenden."""
    for datei in (SRC / "engine").glob("*.rs"):
        quellzeilen = [
            zeile
            for zeile in _lies(datei).splitlines()
            if not zeile.lstrip().startswith(("//", "//!", "///"))
        ]
        code = "\n".join(quellzeilen)
        assert "use tauri" not in code, f"{datei.name} importiert Tauri"
        assert "tauri::" not in code, f"{datei.name} verwendet Tauri"


def test_fortschrittsereignis_hat_einen_stabilen_namen():
    assert 'PROGRESS_EVENT: &str = "plane://progress"' in _lies(COMMANDS_RS)


# --------------------------------------------------------------------------
# Katalog und Übersetzungen
# --------------------------------------------------------------------------


def test_katalog_ist_umfangreich():
    """Ein Cleaner mit einer Handvoll Zielen wäre kein Cleaner."""
    assert len(katalog_schluessel()) >= 30


def test_jedes_ziel_hat_name_und_beschreibung():
    schluessel = i18n_schluessel()
    for ziel in katalog_schluessel():
        for suffix in ("name", "description"):
            key = f"target.{ziel}.{suffix}"
            assert key in schluessel, f"Übersetzung fehlt: {key}"


def test_keine_verwaisten_zieluebersetzungen():
    ziele = katalog_schluessel()
    for key in i18n_schluessel():
        if not key.startswith("target."):
            continue
        rest = key[len("target.") :]
        ziel = re.sub(r"\.(name|description)$", "", rest)
        assert ziel in ziele, f"Übersetzung ohne Ziel: {key}"


def test_alle_kategorien_sind_uebersetzt():
    schluessel = i18n_schluessel()
    for kategorie in ("system", "browsers", "apps", "installers", "recyclebin", "registry"):
        assert f"category.{kategorie}" in schluessel


def test_pfade_verdrahten_kein_laufwerk():
    """Hart verdrahtete Laufwerksbuchstaben brechen auf Systemen, die nicht
    auf C: installiert sind."""
    quelle = _lies(CATALOG_RS).split("#[cfg(test)]", 1)[0]
    treffer = re.findall(r'FileRule::new\("([A-Za-z]:[^"]*)"', quelle)
    assert not treffer, f"Muster mit festem Laufwerk: {treffer}"


def test_registry_verzichtet_auf_die_gefaehrlichen_bereiche():
    """COM/CLSID, TypeLib und Dienste zu bereinigen ist die dokumentierte
    Hauptursache für Systemschäden durch Registry-Cleaner."""
    quelle = _lies(CATALOG_RS).split("#[cfg(test)]", 1)[0]
    for verboten in (r"Classes\CLSID", r"Classes\TypeLib", "CurrentControlSet"):
        assert verboten not in quelle, f"Riskante Registry-Regel: {verboten}"


def test_registry_sichert_vor_dem_loeschen():
    quelle = _lies(SRC / "engine" / "clean.rs")
    assert "registry::backup" in quelle
    assert "error.registry_backup" in quelle


# --------------------------------------------------------------------------
# Tauri-Konfiguration
# --------------------------------------------------------------------------


@pytest.fixture(scope="module")
def tauri_config() -> dict:
    return json.loads(_lies(SRC_TAURI / "tauri.conf.json"))


def test_tauri_capabilities_sind_definiert():
    caps = SRC_TAURI / "capabilities"
    dateien = list(caps.glob("*.json")) if caps.is_dir() else []
    assert dateien, "src-tauri/capabilities/ fehlt oder ist leer."
    for datei in dateien:
        assert json.loads(_lies(datei)).get("permissions")


def test_capability_gilt_fuer_ein_existierendes_fenster(tauri_config: dict):
    fenster = {w["label"] for w in tauri_config["app"]["windows"] if "label" in w}
    for datei in (SRC_TAURI / "capabilities").glob("*.json"):
        for label in json.loads(_lies(datei)).get("windows", []):
            assert label in fenster, f"Capability zeigt auf unbekanntes Fenster: {label}"


def test_csp_ist_gesetzt(tauri_config: dict):
    csp = tauri_config["app"]["security"].get("csp")
    assert csp, "Ohne CSP gibt es keinen Schutz gegen eingeschleuste Skripte."
    assert "script-src 'self'" in csp


def test_frontend_wird_vor_dem_bundle_gebaut(tauri_config: dict):
    """Sonst bündelt `tauri build` einen veralteten Frontend-Stand."""
    assert tauri_config["build"]["beforeBuildCommand"] == "npm run build"
    assert tauri_config["build"]["beforeDevCommand"] == "npm run dev"


def test_schema_verweist_auf_tauri_2(tauri_config: dict):
    assert tauri_config["$schema"] == "https://schema.tauri.app/config/2"


def test_build_datum_ist_nicht_hart_codiert():
    quelle = _lies(COMMANDS_RS)
    assert 'env!("PLANE_BUILD_DATE")' in quelle
    assert not re.search(r'build_date:\s*"\d{4}-', quelle)


# --------------------------------------------------------------------------
# Projekthygiene
# --------------------------------------------------------------------------


def test_lizenzdatei_existiert():
    lizenz = PROJEKT_WURZEL / "LICENSE"
    assert lizenz.exists(), "README verweist auf eine LICENSE-Datei."
    assert "MIT License" in _lies(lizenz)


def test_gitignore_deckt_die_buildartefakte_ab():
    inhalt = _lies(PROJEKT_WURZEL / ".gitignore")
    for eintrag in ("node_modules/", "dist/", "src-tauri/target/", "__pycache__/"):
        assert eintrag in inhalt, f"Nicht ignoriert: {eintrag}"


def test_readme_verweist_auf_die_dokumentation():
    inhalt = _lies(PROJEKT_WURZEL / "README.md")
    for dokument in ("docs/DATENSTRUKTUREN.md", "docs/TESTS.md"):
        assert dokument in inhalt


def test_abhaengigkeiten_sind_deklariert_und_gepinnt():
    for datei in ("requirements.txt", "requirements-dev.txt"):
        for zeile in _lies(PROJEKT_WURZEL / datei).splitlines():
            zeile = zeile.strip()
            if not zeile or zeile.startswith(("#", "-r ")):
                continue
            assert "==" in zeile, f"{datei}: ungepinnt -> {zeile}"


def test_quelldateien_sind_utf8_mit_echten_umlauten():
    """Die Projektsprache ist Deutsch – Umlaute gehören korrekt kodiert."""
    quelle = _lies(I18N_RS)
    assert "ö" in quelle and "ä" in quelle and "ü" in quelle
    assert '"Uebersicht"' not in quelle, "Umlautersatz statt echter Umlaute"
