//! Einstiegspunkt der Plane-Desktopanwendung.
//!
//! Die Logik liegt vollstaendig in der Bibliothek (`lib.rs`), damit sie nur
//! einmal kompiliert und getestet wird.

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

fn main() {
    plane_lib::run();
}
