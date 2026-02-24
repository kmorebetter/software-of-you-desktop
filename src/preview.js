import { invoke } from '@tauri-apps/api/core';
import { convertFileSrc } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

const recents = [];
const MAX_RECENTS = 20;

function relativeTime(date) {
  const seconds = Math.floor((Date.now() - date.getTime()) / 1000);
  if (seconds < 10) return 'just now';
  if (seconds < 60) return `${seconds}s ago`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  return `${Math.floor(seconds / 3600)}h ago`;
}

function renderRecents() {
  const list = document.getElementById('recents-list');
  if (!list) return;

  // Clear existing children safely using DOM API
  while (list.firstChild) {
    list.removeChild(list.firstChild);
  }

  for (const recent of recents) {
    const li = document.createElement('li');

    const nameSpan = document.createElement('span');
    nameSpan.textContent = recent.filename;

    const timeSpan = document.createElement('span');
    timeSpan.className = 'recents-time';
    timeSpan.textContent = relativeTime(recent.loadedAt);

    li.appendChild(nameSpan);
    li.appendChild(timeSpan);
    li.addEventListener('click', () => loadView(recent.path));

    list.appendChild(li);
  }
}

function loadView(filePath) {
  const url = convertFileSrc(filePath);
  const iframe = document.getElementById('preview-frame');
  const empty = document.getElementById('preview-empty');
  const header = document.getElementById('preview-filename');
  const viewInfo = document.getElementById('view-info');

  iframe.src = url;

  // Show iframe, hide empty state
  iframe.style.display = '';
  iframe.classList.remove('hidden');
  if (empty) empty.classList.add('hidden');

  const filename = filePath.split('/').pop();

  // Add to recents (prepend, deduplicate, cap at MAX_RECENTS)
  const existingIdx = recents.findIndex((r) => r.path === filePath);
  if (existingIdx !== -1) recents.splice(existingIdx, 1);
  recents.unshift({ path: filePath, filename, loadedAt: new Date() });
  if (recents.length > MAX_RECENTS) recents.length = MAX_RECENTS;

  // Update header
  if (header) header.textContent = filename;

  // Update status bar
  if (viewInfo) viewInfo.textContent = `${filename} — just now`;

  renderRecents();
}

function toggleRecents() {
  const drawer = document.getElementById('recents-drawer');
  const toggle = document.getElementById('recents-toggle');
  if (!drawer) return;

  drawer.classList.toggle('open');

  if (toggle) {
    const isOpen = drawer.classList.contains('open');
    toggle.textContent = isOpen ? '\u25BC Recents' : '\u25B6 Recents';
  }
}

async function init() {
  try {
    const config = await invoke('get_config');

    // Tilde expansion is handled on the Rust side in start_watcher
    const outputDir = config.output_path;
    await invoke('start_watcher', { outputDir });
  } catch (err) {
    console.error('Failed to initialize preview watcher:', err);
  }

  listen('new-view', (event) => {
    loadView(event.payload);
  });

  // Recents toggle button
  const toggleBtn = document.getElementById('recents-toggle');
  if (toggleBtn) {
    toggleBtn.addEventListener('click', toggleRecents);
  }

  // Refresh relative times every 30 seconds
  setInterval(renderRecents, 30000);
}

document.addEventListener('DOMContentLoaded', init);

// Reload current preview (View > Reload Preview)
export function reloadPreview() {
  const iframe = document.getElementById('preview-frame');
  if (iframe.src && iframe.src !== 'about:blank') {
    iframe.src = iframe.src;
  }
}
