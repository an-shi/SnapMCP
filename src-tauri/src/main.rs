#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if snapmcp_lib::cli::try_run_stdio_bridge() {
        return;
    }
    snapmcp_lib::run();
}
