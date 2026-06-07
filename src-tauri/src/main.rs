// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if qwert_lib::cli::is_cli_mode(&args) {
        std::process::exit(qwert_lib::cli::run());
    } else {
        qwert_lib::run();
    }
}
