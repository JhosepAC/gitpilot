#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if gitpilot_desktop::run().is_err() {
        eprintln!("GitPilot could not start.");
        std::process::exit(1);
    }
}
