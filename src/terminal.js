import '@xterm/xterm/css/xterm.css';
import { Terminal } from '@xterm/xterm';
import { WebglAddon } from '@xterm/addon-webgl';
import { FitAddon } from '@xterm/addon-fit';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

let term;
let fitAddon;
let ptyStarted = false;
let initialized = false;

// Store unlisten functions so we can clean up on HMR reload
let unlistenPtyOutput = null;
let unlistenMenuAction = null;

function updateDimensions() {
  const dims = document.getElementById('pty-dimensions');
  if (dims && term) {
    dims.textContent = `${term.cols}\u00d7${term.rows}`;
  }
}

function setStatus(state, label) {
  const dot = document.getElementById('pty-status-dot');
  const text = document.getElementById('pty-status-text');
  if (dot) dot.className = `status-dot ${state}`;
  if (text) text.textContent = label;
}

async function startPty() {
  if (ptyStarted) return;

  try {
    setStatus('starting', 'starting...');
    const config = await invoke('get_config');

    const claudePath = config.claude_path || 'claude';
    const pluginPath = config.plugin_path_override || config.plugin_path;

    let pathEnv = '';
    if (claudePath && claudePath.includes('/')) {
      const claudeDir = claudePath.substring(0, claudePath.lastIndexOf('/'));
      const defaultPath = '/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin';
      pathEnv = `${claudeDir}:${defaultPath}`;
    }

    // Ensure layout is settled and fit is current before reading dimensions
    fitAddon.fit();

    await invoke('pty_start', {
      projectPath: pluginPath,
      pathEnv,
      cols: term.cols,
      rows: term.rows,
    });

    ptyStarted = true;
    setStatus('running', 'claude');

    // Send an immediate resize to guarantee PTY matches xterm after start
    await invoke('pty_resize', { cols: term.cols, rows: term.rows }).catch(() => {});
    updateDimensions();
    term.focus();
  } catch (err) {
    console.error('Failed to start PTY:', err);
    setStatus('stopped', 'error');
  }
}

async function initTerminal() {
  // Guard against double init from HMR
  if (initialized) return;
  initialized = true;

  term = new Terminal({
    fontFamily: '"JetBrains Mono", "SF Mono", monospace',
    fontSize: 13,
    lineHeight: 1.4,
    theme: {
      background: '#0f1117',
      foreground: '#e2e8f0',
      cursor: '#f97316',
      selectionBackground: 'rgba(249, 115, 22, 0.3)',
    },
    scrollback: 10000,
    convertEol: false,
    cursorBlink: true,
    cursorStyle: 'bar',
  });

  fitAddon = new FitAddon();
  term.loadAddon(fitAddon);

  term.open(document.getElementById('terminal'));

  try {
    term.loadAddon(new WebglAddon());
  } catch (e) {
    console.warn('WebGL not available, using canvas renderer');
  }

  fitAddon.fit();
  updateDimensions();

  const ro = new ResizeObserver(() => {
    fitAddon.fit();
    updateDimensions();
    if (ptyStarted) {
      invoke('pty_resize', { cols: term.cols, rows: term.rows }).catch(() => {});
    }
  });
  ro.observe(document.getElementById('terminal'));

  // Clean up old listeners if they exist (HMR safety)
  if (unlistenPtyOutput) unlistenPtyOutput();
  if (unlistenMenuAction) unlistenMenuAction();

  unlistenPtyOutput = await listen('pty-output', (event) => {
    term.write(event.payload);
  });

  unlistenMenuAction = await listen('menu-action', (event) => {
    switch (event.payload) {
      case 'clear-terminal':
        term.clear();
        break;
      case 'toggle-preview':
        document.getElementById('preview-panel').classList.toggle('hidden');
        fitAddon.fit();
        break;
      case 'reload-preview': {
        const iframe = document.getElementById('preview-frame');
        if (iframe && iframe.src && iframe.src !== 'about:blank') {
          iframe.src = iframe.src;
        }
        break;
      }
    }
  });

  term.onData((data) => {
    if (ptyStarted) {
      invoke('pty_write', { data });
    }
  });

  // Panel resize via divider drag
  const divider = document.getElementById('divider');
  if (divider) {
    let dragging = false;
    divider.addEventListener('mousedown', (e) => {
      dragging = true;
      e.preventDefault();
    });
    window.addEventListener('mousemove', (e) => {
      if (!dragging) return;
      const panels = document.getElementById('panels');
      const rect = panels.getBoundingClientRect();
      const pct = ((e.clientX - rect.left) / rect.width) * 100;
      const clamped = Math.max(20, Math.min(80, pct));
      document.getElementById('terminal-panel').style.width = `${clamped}%`;
      document.getElementById('preview-panel').style.width = `${100 - clamped}%`;
      fitAddon.fit();
      updateDimensions();
    });
    window.addEventListener('mouseup', () => {
      dragging = false;
    });
  }

  // Check if onboarding is already complete — if so, start PTY immediately
  try {
    const status = await invoke('check_setup');
    if (status.onboarding_complete) {
      await startPty();
    }
  } catch (err) {
    console.error('Failed to check setup:', err);
  }
}

document.addEventListener('DOMContentLoaded', initTerminal);

export async function startPtyAfterOnboarding() {
  await startPty();
}

export function clearTerminal() {
  if (term) term.clear();
}

export function focusTerminal() {
  if (term) term.focus();
}
