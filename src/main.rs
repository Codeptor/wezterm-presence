// Hide console window in release builds on Windows
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod presence;
mod wezterm;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, TrayIconBuilder};

/// Generate a simple 16x16 pink icon (matches WezTerm theme).
fn create_icon() -> Icon {
    let size = 16u32;
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            // Circle mask
            let cx = (x as f32) - 7.5;
            let cy = (y as f32) - 7.5;
            let dist = (cx * cx + cy * cy).sqrt();
            if dist < 7.0 {
                // #ff6b9d - the pink from the WezTerm theme
                rgba.extend_from_slice(&[0xff, 0x6b, 0x9d, 0xff]);
            } else {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }
    Icon::from_rgba(rgba, size, size).expect("Failed to create icon")
}

fn run_presence_loop(running: Arc<AtomicBool>) {
    let config = config::Config::load();
    let _poll_interval = Duration::from_secs(config.poll_interval);

    let mut presence = match presence::Presence::new(&config.discord_app_id) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to create Discord IPC client: {}", e);
            return;
        }
    };

    let mut was_running = false;
    let mut last_state: Option<wezterm::TerminalState> = None;

    while running.load(Ordering::Relaxed) {
        if !presence.connect() {
            for _ in 0..10 {
                if !running.load(Ordering::Relaxed) {
                    break;
                }
                thread::sleep(Duration::from_secs(1));
            }
            continue;
        }

        match wezterm::poll() {
            Some(state) => {
                if !was_running {
                    presence.reset_session_timer();
                    was_running = true;
                }
                // Only update Discord if state actually changed
                if last_state.as_ref() != Some(&state) {
                    presence.update(&state, &config);
                    last_state = Some(state);
                }
            }
            None => {
                if was_running {
                    presence.clear();
                    was_running = false;
                    last_state = None;
                }
            }
        }

        // Sleep in small increments so we can check the running flag
        for _ in 0..config.poll_interval {
            if !running.load(Ordering::Relaxed) {
                break;
            }
            thread::sleep(Duration::from_secs(1));
        }
    }

    // Clean up: clear presence before exiting
    presence.clear();
}

fn main() {
    // Handle `init` subcommand: generate default config.toml
    if std::env::args().nth(1).as_deref() == Some("init") {
        let path = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join("config.toml")))
            .unwrap_or_else(|| std::path::PathBuf::from("config.toml"));
        if path.exists() {
            eprintln!("config.toml already exists at {}", path.display());
            std::process::exit(1);
        }
        config::Config::write_default(&path).expect("Failed to write config.toml");
        eprintln!("Created default config at {}", path.display());
        eprintln!("Edit it to set your discord_app_id, then run wezterm-presence.");
        return;
    }

    // Handle `install` subcommand: create startup shortcut
    if std::env::args().nth(1).as_deref() == Some("install") {
        install_startup();
        return;
    }

    // Set up system tray
    let menu = Menu::new();
    let quit_item = MenuItem::new("Quit", true, None);
    menu.append(&quit_item).expect("Failed to build menu");

    let _tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("WezTerm Presence")
        .with_icon(create_icon())
        .build()
        .expect("Failed to create tray icon");

    // Start the presence loop in a background thread
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();
    let worker = thread::spawn(move || {
        run_presence_loop(running_clone);
    });

    // Main thread: pump Windows messages and handle tray events
    let menu_receiver = MenuEvent::receiver();

    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            DispatchMessageW, GetMessageW, TranslateMessage, MSG,
        };

        loop {
            unsafe {
                let mut msg: MSG = std::mem::zeroed();
                let ret = GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0);
                if ret <= 0 {
                    running.store(false, Ordering::Relaxed);
                    break;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            if let Ok(event) = menu_receiver.try_recv() {
                if event.id == quit_item.id() {
                    running.store(false, Ordering::Relaxed);
                    break;
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        loop {
            if let Ok(event) = menu_receiver.try_recv() {
                if event.id == quit_item.id() {
                    running.store(false, Ordering::Relaxed);
                    break;
                }
            }
            thread::sleep(Duration::from_millis(100));
        }
    }

    // Wait for worker to clear presence before exiting
    let _ = worker.join();
}

fn install_startup() {
    let exe_path = std::env::current_exe().expect("Failed to get executable path");

    let startup_dir = dirs::config_dir()
        .map(|p| {
            p.join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs")
                .join("Startup")
        })
        .expect("Failed to find startup directory");

    let vbs_path = startup_dir.join("wezterm-presence.vbs");
    let vbs_content = format!(
        "Set WshShell = CreateObject(\"WScript.Shell\")\nWshShell.Run \"\"\"{}\"\"\", 0, False",
        exe_path.display()
    );

    std::fs::write(&vbs_path, vbs_content).expect("Failed to write startup script");
    eprintln!("Installed auto-start to: {}", vbs_path.display());
    eprintln!("wezterm-presence will start automatically on login.");
}
