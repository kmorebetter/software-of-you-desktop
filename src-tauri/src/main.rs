#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;

use tauri::Emitter;
use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder};

mod config;
mod pty;
mod setup;
mod watcher;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(pty::PtyState::default()))
        .manage(Mutex::new(watcher::WatcherState::default()))
        .invoke_handler(tauri::generate_handler![
            pty::pty_start,
            pty::pty_write,
            pty::pty_resize,
            pty::pty_stop,
            watcher::start_watcher,
            config::get_config,
            config::save_config,
            setup::check_setup,
            setup::run_onboarding_step1,
            setup::run_onboarding_step2,
        ])
        .setup(|app| {
            setup_menu(app)?;

            if let Err(e) = setup::extract_plugin_if_needed(&app.handle()) {
                eprintln!("Plugin extraction check failed: {}", e);
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn main() {
    run();
}

fn setup_menu(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle();

    // App submenu
    let about = PredefinedMenuItem::about(handle, Some("About SoY"), None)?;
    let separator1 = PredefinedMenuItem::separator(handle)?;
    let settings = MenuItemBuilder::with_id("settings", "Settings...")
        .accelerator("CmdOrCtrl+,")
        .build(handle)?;
    let separator2 = PredefinedMenuItem::separator(handle)?;
    let quit = PredefinedMenuItem::quit(handle, Some("Quit SoY"))?;

    let app_menu = SubmenuBuilder::new(handle, "Software of You")
        .items(&[&about, &separator1, &settings, &separator2, &quit])
        .build()?;

    // View submenu
    let reload_preview = MenuItemBuilder::with_id("reload_preview", "Reload Preview")
        .accelerator("CmdOrCtrl+R")
        .build(handle)?;
    let clear_terminal = MenuItemBuilder::with_id("clear_terminal", "Clear Terminal")
        .accelerator("CmdOrCtrl+K")
        .build(handle)?;
    let separator3 = PredefinedMenuItem::separator(handle)?;
    let toggle_preview = MenuItemBuilder::with_id("toggle_preview", "Toggle Preview")
        .accelerator("CmdOrCtrl+\\")
        .build(handle)?;
    let separator4 = PredefinedMenuItem::separator(handle)?;
    let zoom_in = MenuItemBuilder::with_id("zoom_in", "Zoom In")
        .accelerator("CmdOrCtrl+=")
        .build(handle)?;
    let zoom_out = MenuItemBuilder::with_id("zoom_out", "Zoom Out")
        .accelerator("CmdOrCtrl+-")
        .build(handle)?;
    let reset_zoom = MenuItemBuilder::with_id("reset_zoom", "Reset Zoom")
        .accelerator("CmdOrCtrl+0")
        .build(handle)?;

    let view_menu = SubmenuBuilder::new(handle, "View")
        .items(&[
            &reload_preview,
            &clear_terminal,
            &separator3,
            &toggle_preview,
            &separator4,
            &zoom_in,
            &zoom_out,
            &reset_zoom,
        ])
        .build()?;

    // Window submenu
    let minimize = PredefinedMenuItem::minimize(handle, Some("Minimize"))?;
    let zoom = PredefinedMenuItem::fullscreen(handle, Some("Zoom"))?;
    let separator5 = PredefinedMenuItem::separator(handle)?;
    let bring_all =
        PredefinedMenuItem::bring_all_to_front(handle, Some("Bring All to Front"))?;

    let window_menu = SubmenuBuilder::new(handle, "Window")
        .items(&[&minimize, &zoom, &separator5, &bring_all])
        .build()?;

    // Build full menu
    let menu = MenuBuilder::new(handle)
        .items(&[&app_menu, &view_menu, &window_menu])
        .build()?;

    app.set_menu(menu)?;

    // Handle menu events — emit to frontend via "menu-action" event
    app.on_menu_event(move |app_handle, event| {
        match event.id().as_ref() {
            "reload_preview" => {
                app_handle.emit("menu-action", "reload-preview").ok();
            }
            "clear_terminal" => {
                app_handle.emit("menu-action", "clear-terminal").ok();
            }
            "toggle_preview" => {
                app_handle.emit("menu-action", "toggle-preview").ok();
            }
            "settings" => {
                app_handle.emit("menu-action", "settings").ok();
            }
            _ => {}
        }
    });

    Ok(())
}
