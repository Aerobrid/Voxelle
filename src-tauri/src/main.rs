// Voxelle — local, private-first offline dictation & voice AI.
// Hide the console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    voxelle_core::run();
}
