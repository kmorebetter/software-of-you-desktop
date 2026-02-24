use portable_pty::{CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::sync::Mutex;
use tauri::Emitter;

fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return format!("{}{}", home.display(), &path[1..]);
        }
    }
    path.to_string()
}

pub struct PtyState {
    pub writer: Option<Box<dyn Write + Send>>,
    pub master: Option<Box<dyn MasterPty + Send>>,
    pub reader_thread: Option<std::thread::JoinHandle<()>>,
    pub child: Option<Box<dyn portable_pty::Child + Send + Sync>>,
}

impl Default for PtyState {
    fn default() -> Self {
        Self {
            writer: None,
            master: None,
            reader_thread: None,
            child: None,
        }
    }
}

#[tauri::command]
pub fn pty_start(
    project_path: String,
    path_env: String,
    cols: u16,
    rows: u16,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, Mutex<PtyState>>,
) -> Result<(), String> {
    // Stop existing PTY if one is already running to prevent resource leaks
    {
        let mut pty = state.lock().map_err(|e| e.to_string())?;
        if pty.child.is_some() {
            if let Some(mut child) = pty.child.take() {
                let _ = child.kill();
            }
            pty.writer.take();
            pty.master.take();
            if let Some(thread) = pty.reader_thread.take() {
                let _ = thread.join();
            }
        }
    }

    let project_path = expand_tilde(&project_path);
    let pty_system = portable_pty::native_pty_system();

    let size = PtySize {
        rows: if rows > 0 { rows } else { 24 },
        cols: if cols > 0 { cols } else { 80 },
        pixel_width: 0,
        pixel_height: 0,
    };

    let pair = pty_system.openpty(size).map_err(|e| e.to_string())?;

    let mut cmd = CommandBuilder::new("claude");
    cmd.env("CLAUDE_PLUGIN_ROOT", &project_path);
    cmd.env("TERM", "xterm-256color");

    if !path_env.is_empty() {
        cmd.env("PATH", &path_env);
    } else if let Ok(sys_path) = std::env::var("PATH") {
        cmd.env("PATH", sys_path);
    }

    if let Ok(home) = std::env::var("HOME") {
        cmd.env("HOME", home);
    }

    cmd.cwd(&project_path);

    let child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;

    // Slave is no longer needed after spawning
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
    let writer = pair.master.take_writer().map_err(|e| e.to_string())?;

    let reader_thread = std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let data = String::from_utf8_lossy(&buf[..n]).to_string();
                    let _ = app_handle.emit("pty-output", &data);
                }
                Err(_) => break,
            }
        }
    });

    let mut pty = state.lock().map_err(|e| e.to_string())?;
    pty.writer = Some(writer);
    pty.master = Some(pair.master);
    pty.reader_thread = Some(reader_thread);
    pty.child = Some(child);

    Ok(())
}

#[tauri::command]
pub fn pty_write(data: String, state: tauri::State<'_, Mutex<PtyState>>) -> Result<(), String> {
    let mut pty = state.lock().map_err(|e| e.to_string())?;
    let writer = pty
        .writer
        .as_mut()
        .ok_or_else(|| "PTY not started".to_string())?;
    writer
        .write_all(data.as_bytes())
        .map_err(|e| e.to_string())?;
    writer.flush().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn pty_resize(
    cols: u16,
    rows: u16,
    state: tauri::State<'_, Mutex<PtyState>>,
) -> Result<(), String> {
    let pty = state.lock().map_err(|e| e.to_string())?;
    let master = pty
        .master
        .as_ref()
        .ok_or_else(|| "PTY not started".to_string())?;
    master
        .resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn pty_stop(state: tauri::State<'_, Mutex<PtyState>>) -> Result<(), String> {
    let mut pty = state.lock().map_err(|e| e.to_string())?;

    if let Some(mut child) = pty.child.take() {
        let _ = child.kill();
    }

    // Drop writer and master to close the PTY, which unblocks the reader thread
    pty.writer.take();
    pty.master.take();

    if let Some(thread) = pty.reader_thread.take() {
        let _ = thread.join();
    }

    Ok(())
}
