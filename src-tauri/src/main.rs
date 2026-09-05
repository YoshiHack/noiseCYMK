// Prevent additional console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Lib crate name is `noise_cymk` (snake_case required by Rust); binary
    // name is `noiseCYMK` (the brand). lib.rs re-exports under that alias.
    noise_cymk::run();
}