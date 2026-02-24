use serde::{Deserialize, Serialize};
use std::fs;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SetupStatus {
    pub onboarding_complete: bool,
    pub claude_path: Option<String>,
    pub plugin_path: Option<String>,
}

fn plugin_dest() -> Result<PathBuf, String> {
    dirs::data_dir()
        .map(|d| d.join("software-of-you").join("plugin"))
        .ok_or_else(|| "Could not determine data directory".to_string())
}

/// Find the plugin source directory. Checks the Tauri resource dir first (bundled app),
/// then falls back to the project-relative `plugin/` directory (dev mode).
fn find_plugin_source(app_handle: &AppHandle) -> Result<PathBuf, String> {
    // Bundled mode: resource_dir/plugin
    if let Ok(res_dir) = app_handle.path().resource_dir() {
        let bundled = res_dir.join("plugin");
        if bundled.exists() && bundled.join("VERSION").exists() {
            return Ok(bundled);
        }
    }

    // Dev mode: look relative to the binary's ancestor directories for plugin/
    if let Ok(exe) = std::env::current_exe() {
        // Walk up from the binary location looking for plugin/VERSION
        let mut dir = exe.parent().map(|p| p.to_path_buf());
        for _ in 0..10 {
            if let Some(ref d) = dir {
                let candidate = d.join("plugin");
                if candidate.join("VERSION").exists() {
                    return Ok(candidate);
                }
                dir = d.parent().map(|p| p.to_path_buf());
            } else {
                break;
            }
        }
    }

    // Last resort: current working directory
    if let Ok(cwd) = std::env::current_dir() {
        let candidate = cwd.join("plugin");
        if candidate.join("VERSION").exists() {
            return Ok(candidate);
        }
    }

    Err("Plugin source not found. Ensure plugin/ directory exists with a VERSION file.".to_string())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("Failed to create dir {}: {}", dst.display(), e))?;

    let entries =
        fs::read_dir(src).map_err(|e| format!("Failed to read dir {}: {}", src.display(), e))?;

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let file_type = entry.file_type().map_err(|e| e.to_string())?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| {
                format!(
                    "Failed to copy {} -> {}: {}",
                    src_path.display(),
                    dst_path.display(),
                    e
                )
            })?;
        }
    }

    Ok(())
}

#[tauri::command]
pub fn check_setup() -> Result<SetupStatus, String> {
    let config_file = crate::config::config_path()?;

    if config_file.exists() {
        let contents = fs::read_to_string(&config_file).map_err(|e| e.to_string())?;
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&contents) {
            if val.get("onboarding_complete") == Some(&serde_json::Value::Bool(true)) {
                return Ok(SetupStatus {
                    onboarding_complete: true,
                    claude_path: val
                        .get("claude_path")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    plugin_path: val
                        .get("plugin_path")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                });
            }
        }
    }

    let claude_path = std::process::Command::new("which")
        .arg("claude")
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        });

    let dest = plugin_dest()?;
    let plugin_path = if dest.exists() {
        Some(dest.to_string_lossy().to_string())
    } else {
        None
    };

    Ok(SetupStatus {
        onboarding_complete: false,
        claude_path,
        plugin_path,
    })
}

#[tauri::command]
pub fn run_onboarding_step1() -> Result<String, String> {
    let output = std::process::Command::new("which")
        .arg("claude")
        .output()
        .map_err(|e| format!("Failed to run 'which claude': {}", e))?;

    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(path)
    } else {
        Err("Claude Code not found. Install from https://claude.ai/download".to_string())
    }
}

#[tauri::command]
pub fn run_onboarding_step2(
    claude_path: String,
    app_handle: AppHandle,
) -> Result<(), String> {
    let dest = plugin_dest()?;
    let source = find_plugin_source(&app_handle)?;

    copy_dir_recursive(&source, &dest)?;

    let bootstrap_script = dest.join("shared").join("bootstrap.sh");
    if !bootstrap_script.exists() {
        return Err(format!(
            "Bootstrap script not found at {}",
            bootstrap_script.display()
        ));
    }

    let mut child = std::process::Command::new("bash")
        .arg(&bootstrap_script)
        .env("CLAUDE_PLUGIN_ROOT", &dest)
        .env("CLAUDE_PATH", &claude_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to run bootstrap: {}", e))?;

    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let handle = app_handle.clone();

    let reader_thread = std::thread::spawn(move || {
        let reader = std::io::BufReader::new(stdout);
        let mut saw_ready = false;
        for line in reader.lines() {
            if let Ok(line) = line {
                if line.starts_with("ready|") {
                    saw_ready = true;
                }
                let _ = handle.emit("bootstrap-output", &line);
            }
        }
        saw_ready
    });

    let stderr = child.stderr.take();
    let stderr_thread = std::thread::spawn(move || {
        if let Some(stderr) = stderr {
            let reader = std::io::BufReader::new(stderr);
            let mut err_output = String::new();
            for line in reader.lines() {
                if let Ok(line) = line {
                    err_output.push_str(&line);
                    err_output.push('\n');
                }
            }
            err_output
        } else {
            String::new()
        }
    });

    let status = child.wait().map_err(|e| format!("Bootstrap process error: {}", e))?;
    let saw_ready = reader_thread.join().unwrap_or(false);
    let stderr_output = stderr_thread.join().unwrap_or_default();

    if !status.success() {
        return Err(format!(
            "Bootstrap exited with code {:?}. stderr: {}",
            status.code(),
            stderr_output.trim()
        ));
    }

    if !saw_ready {
        return Err("Bootstrap completed but never emitted ready signal".to_string());
    }

    Ok(())
}

pub fn extract_plugin_if_needed(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let dest = plugin_dest()?;
    let source = match find_plugin_source(app_handle) {
        Ok(s) => s,
        Err(_) => return Ok(dest), // No plugin source found — skip extraction silently
    };

    let bundle_version = fs::read_to_string(source.join("VERSION"))
        .unwrap_or_default()
        .trim()
        .to_string();

    let installed_version = fs::read_to_string(dest.join("VERSION"))
        .unwrap_or_default()
        .trim()
        .to_string();

    let needs_extract =
        !dest.exists() || bundle_version != installed_version || installed_version.is_empty();

    if needs_extract && source.exists() {
        if dest.exists() {
            fs::remove_dir_all(&dest)
                .map_err(|e| format!("Failed to remove old plugin: {}", e))?;
        }
        copy_dir_recursive(&source, &dest)?;
    }

    Ok(dest)
}
