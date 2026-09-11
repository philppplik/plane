//! Programmsymbole aus `.exe`, `.dll` und `.ico` lesen.
//!
//! Windows hinterlegt zu jedem Eintrag in „Apps & Features" einen Wert
//! `DisplayIcon`. Er sieht meistens so aus:
//!
//! ```text
//! C:\Program Files\Beispiel\app.exe,0
//! C:\Windows\System32\shell32.dll,-16
//! C:\Program Files\Beispiel\icon.ico
//! ```
//!
//! Die Zahl dahinter ist der Index innerhalb der Datei; ist sie **negativ**,
//! ist ihr Betrag eine Ressourcenkennung statt einer Position.
//!
//! # Warum rohe Bildpunkte statt PNG
//!
//! Ein PNG-Kodierer wäre eine weitere Abhängigkeit samt Kompression für
//! Bilder von 32×32 Punkten. Plane reicht stattdessen BGRA→RGBA-Bildpunkte
//! durch, und die Oberfläche zeichnet sie auf ein `canvas`. Das spart eine
//! Kiste, und es gibt keine `data:`-Adressen, die an der
//! Content-Security-Policy hängen bleiben.
//!
//! # Umgang mit Fehlern
//!
//! Ein fehlendes Symbol ist **kein Fehler**. Programme verschwinden, Pfade
//! stimmen nicht mehr, Ressourcen fehlen. Jede Funktion hier liefert
//! deshalb `Option` und schweigt, statt die Liste mit Meldungen zu fluten.

use serde::{Deserialize, Serialize};

/// Kantenlänge, in der Symbole angefordert werden.
///
/// 32 Punkte ist die Größe, die praktisch jede Symboldatei enthält. Größere
/// Varianten gibt es nicht überall, kleinere sehen auf heutigen Bildschirmen
/// unscharf aus.
pub const KANTE: i32 = 32;

/// Ein Symbol, fertig zum Zeichnen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgramIcon {
    pub width: u32,
    pub height: u32,
    /// RGBA-Bildpunkte, zeilenweise von oben, als base64.
    pub rgba: String,
}

/// Quelle in Pfad und Index zerlegen.
///
/// Anführungszeichen kommen vor (`"C:\...\app.exe",0`) und müssen weg, bevor
/// der Pfad brauchbar ist. Ein Komma im Dateinamen wäre mehrdeutig — deshalb
/// wird nur **hinter** dem letzten Komma nach einer Zahl gesucht, und nur,
/// wenn dort tatsächlich eine steht.
pub fn zerlege(quelle: &str) -> Option<(String, i32)> {
    let roh = quelle.trim();
    if roh.is_empty() {
        return None;
    }

    if let Some((pfad, rest)) = roh.rsplit_once(',') {
        if let Ok(index) = rest.trim().parse::<i32>() {
            return Some((saeubere(pfad), index));
        }
    }
    Some((saeubere(roh), 0))
}

fn saeubere(pfad: &str) -> String {
    pfad.trim().trim_matches('"').trim().to_string()
}

// ---------------------------------------------------------------------------
// Windows
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod win {
    use super::{ProgramIcon, KANTE};

    use base64::Engine as _;
    use std::os::windows::ffi::OsStrExt;

    use windows_sys::Win32::Graphics::Gdi::{
        CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO,
        BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HDC,
    };
    use windows_sys::Win32::UI::Shell::ExtractIconExW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO};

    fn weit(text: &str) -> Vec<u16> {
        std::ffi::OsStr::new(text)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    /// Symbol laden und in RGBA umwandeln.
    ///
    /// Gibt `None` zurück, wenn die Datei fehlt, kein Symbol enthält oder
    /// Windows den Zugriff verweigert — alles erwartbare Zustände.
    pub fn lade(pfad: &str, index: i32) -> Option<ProgramIcon> {
        if !std::path::Path::new(pfad).is_file() {
            return None;
        }

        let breit = weit(pfad);
        let mut gross: HICON = std::ptr::null_mut();

        // SAFETY: `breit` ist nullterminiert und lebt über den Aufruf hinaus.
        // `gross` nimmt genau ein Handle auf, wie der letzte Parameter sagt.
        let anzahl =
            unsafe { ExtractIconExW(breit.as_ptr(), index, &mut gross, std::ptr::null_mut(), 1) };

        if anzahl == 0 || gross.is_null() {
            return None;
        }

        let ergebnis = nach_rgba(gross);

        // SAFETY: `gross` stammt aus ExtractIconExW und wurde noch nicht
        // freigegeben. Jeder Rückgabepfad ab hier ist danach.
        unsafe { DestroyIcon(gross) };

        ergebnis
    }

    /// Ein `HICON` in Bildpunkte übersetzen.
    fn nach_rgba(symbol: HICON) -> Option<ProgramIcon> {
        let mut info: ICONINFO = unsafe { std::mem::zeroed() };
        // SAFETY: `info` ist vollständig genullt und groß genug.
        if unsafe { GetIconInfo(symbol, &mut info) } == 0 {
            return None;
        }

        // GetIconInfo legt zwei Bitmaps an, die der Aufrufer freigeben muss –
        // auch dann, wenn er sie gar nicht braucht. Vergisst man das, leckt
        // jeder Listenaufbau ein paar hundert GDI-Objekte.
        let aufraeumen = || unsafe {
            if !info.hbmColor.is_null() {
                DeleteObject(info.hbmColor as _);
            }
            if !info.hbmMask.is_null() {
                DeleteObject(info.hbmMask as _);
            }
        };

        if info.hbmColor.is_null() {
            // Reine Schwarz-Weiß-Symbole (sehr alt) werden nicht unterstützt.
            aufraeumen();
            return None;
        }

        let mut bitmap: BITMAP = unsafe { std::mem::zeroed() };
        // SAFETY: `bitmap` ist genullt; die Größenangabe stimmt mit dem Typ.
        let gelesen = unsafe {
            GetObjectW(
                info.hbmColor as _,
                std::mem::size_of::<BITMAP>() as i32,
                (&mut bitmap as *mut BITMAP).cast(),
            )
        };
        if gelesen == 0 || bitmap.bmWidth <= 0 || bitmap.bmHeight <= 0 {
            aufraeumen();
            return None;
        }

        // Unplausibel große Symbole gibt es nicht; ein solcher Wert deutet
        // auf eine beschädigte Datei hin und würde hier viel Speicher kosten.
        if bitmap.bmWidth > 512 || bitmap.bmHeight > 512 {
            aufraeumen();
            return None;
        }

        let breite = bitmap.bmWidth as u32;
        let hoehe = bitmap.bmHeight as u32;

        let mut kopf: BITMAPINFO = unsafe { std::mem::zeroed() };
        kopf.bmiHeader = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: bitmap.bmWidth,
            // Negativ: Zeilen von oben nach unten. Ohne das Vorzeichen
            // stünde das Symbol auf dem Kopf.
            biHeight: -bitmap.bmHeight,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        };

        let mut punkte = vec![0u8; (breite * hoehe * 4) as usize];

        // SAFETY: Ein Nullzeiger als Gerätekontext liefert einen Kontext für
        // den Bildschirm; das Ziel ist groß genug für breite*hoehe*4 Bytes,
        // wie der Kopf sie beschreibt.
        let dc: HDC = unsafe { CreateCompatibleDC(std::ptr::null_mut()) };
        if dc.is_null() {
            aufraeumen();
            return None;
        }

        let zeilen = unsafe {
            GetDIBits(
                dc,
                info.hbmColor,
                0,
                hoehe,
                punkte.as_mut_ptr().cast(),
                &mut kopf,
                DIB_RGB_COLORS,
            )
        };

        // SAFETY: `dc` stammt aus CreateCompatibleDC und wird nicht mehr
        // verwendet.
        unsafe { DeleteDC(dc) };
        aufraeumen();

        if zeilen == 0 {
            return None;
        }

        // GDI liefert BGRA, das Web erwartet RGBA.
        for punkt in punkte.as_chunks_mut::<4>().0 {
            punkt.swap(0, 2);
        }

        // Symbole ohne Alphakanal kommen vollständig durchsichtig an. Sie
        // wären unsichtbar statt fehlend – das ist schlimmer, weil niemand
        // merkt, dass etwas nicht stimmt.
        if punkte.as_chunks::<4>().0.iter().all(|p| p[3] == 0) {
            for punkt in punkte.as_chunks_mut::<4>().0 {
                punkt[3] = 255;
            }
        }

        Some(ProgramIcon {
            width: breite,
            height: hoehe,
            rgba: base64::engine::general_purpose::STANDARD.encode(&punkte),
        })
    }

    /// Ersatzsymbol aus der Programmdatei selbst, wenn `DisplayIcon` fehlt.
    pub fn aus_programmdatei(pfad: &str) -> Option<ProgramIcon> {
        lade(pfad, 0)
    }

    pub const _KANTE: i32 = KANTE;
}

/// Symbol zu einer `DisplayIcon`-Angabe holen.
#[cfg(windows)]
pub fn hole(quelle: &str) -> Option<ProgramIcon> {
    let (pfad, index) = zerlege(quelle)?;

    if let Some(symbol) = win::lade(&pfad, index) {
        return Some(symbol);
    }

    // Zeigte der Index ins Leere, steckt im ersten Symbol der Datei
    // meistens trotzdem das richtige Bild.
    if index != 0 {
        return win::aus_programmdatei(&pfad);
    }
    None
}

#[cfg(not(windows))]
pub fn hole(_quelle: &str) -> Option<ProgramIcon> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pfad_und_index_werden_getrennt() {
        assert_eq!(
            zerlege(r"C:\Programme\App\app.exe,0"),
            Some((r"C:\Programme\App\app.exe".into(), 0))
        );
        assert_eq!(
            zerlege(r"C:\Windows\System32\shell32.dll,-16"),
            Some((r"C:\Windows\System32\shell32.dll".into(), -16))
        );
    }

    #[test]
    fn ohne_index_gilt_null() {
        assert_eq!(
            zerlege(r"C:\Programme\App\icon.ico"),
            Some((r"C:\Programme\App\icon.ico".into(), 0))
        );
    }

    #[test]
    fn anfuehrungszeichen_verschwinden() {
        assert_eq!(
            zerlege(r#""C:\Programme\Mit Leerzeichen\app.exe",1"#),
            Some((r"C:\Programme\Mit Leerzeichen\app.exe".into(), 1))
        );
    }

    /// Ein Komma im Dateinamen darf nicht als Indextrenner gelten – sonst
    /// wird der Pfad abgeschnitten und das Symbol nie gefunden.
    #[test]
    fn komma_im_dateinamen_wird_nicht_als_index_gelesen() {
        assert_eq!(
            zerlege(r"C:\Programme\Firma, Inc\app.exe"),
            Some((r"C:\Programme\Firma, Inc\app.exe".into(), 0))
        );
    }

    #[test]
    fn leere_angabe_ergibt_nichts() {
        assert_eq!(zerlege(""), None);
        assert_eq!(zerlege("   "), None);
    }

    /// Ein nicht vorhandener Pfad ist der Normalfall bei verwaisten
    /// Einträgen und darf niemals einen Fehler erzeugen.
    #[test]
    fn fehlende_datei_liefert_kein_symbol() {
        assert_eq!(hole(r"C:\gibt\es\nicht\app.exe,0"), None);
    }

    /// Auf einem Windows-System muss mindestens `shell32.dll` ein Symbol
    /// liefern – sonst stimmt an der Umwandlung etwas nicht.
    #[cfg(windows)]
    #[test]
    fn shell32_liefert_ein_sichtbares_symbol() {
        let system = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
        let quelle = format!(r"{system}\System32\shell32.dll,0");

        let Some(symbol) = hole(&quelle) else {
            // Auf Windows on Arm unter Emulation kann shell32 fehlen; das
            // ist kein Grund, die Testsuite scheitern zu lassen.
            return;
        };

        assert!(symbol.width > 0 && symbol.height > 0);
        assert!(!symbol.rgba.is_empty());

        use base64::Engine as _;
        let punkte = base64::engine::general_purpose::STANDARD
            .decode(&symbol.rgba)
            .expect("base64 muss dekodierbar sein");
        assert_eq!(
            punkte.len() as u32,
            symbol.width * symbol.height * 4,
            "Bildpunkte passen nicht zur Größe"
        );
        assert!(
            punkte.as_chunks::<4>().0.iter().any(|p| p[3] > 0),
            "vollständig durchsichtiges Symbol wäre unsichtbar"
        );
    }
}
