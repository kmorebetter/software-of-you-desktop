import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { startPtyAfterOnboarding } from './terminal.js';

let currentClaudePath = null;

function setStepActive(step) {
  const dots = document.querySelectorAll('.step-dot');
  dots.forEach((dot, i) => {
    dot.classList.remove('active', 'completed');
    if (i + 1 < step) dot.classList.add('completed');
    if (i + 1 === step) dot.classList.add('active');
  });
}

function appendBootstrapLine(line) {
  const log = document.getElementById('bootstrap-log');
  if (!log) return;
  const div = document.createElement('div');
  div.textContent = line;
  log.appendChild(div);
  log.scrollTop = log.scrollHeight;
}

function clearChildren(el) {
  while (el.firstChild) {
    el.removeChild(el.firstChild);
  }
}

function createResultContainer(type) {
  const wrapper = document.createElement('div');
  wrapper.className = `onboarding-result ${type}`;

  const icon = document.createElement('span');
  icon.className = `result-icon ${type}-icon`;
  icon.textContent = type === 'success' ? '\u2713' : '\u2717';
  wrapper.appendChild(icon);

  return wrapper;
}

function showStep1Success(path) {
  const content = document.getElementById('step-1-content');
  if (!content) return;
  clearChildren(content);

  const wrapper = createResultContainer('success');

  const text = document.createElement('p');
  text.className = 'result-text';
  text.textContent = 'Claude Code found';
  wrapper.appendChild(text);

  const detail = document.createElement('p');
  detail.className = 'result-detail muted';
  detail.textContent = path;
  wrapper.appendChild(detail);

  content.appendChild(wrapper);
}

function showStep1Error(err) {
  const content = document.getElementById('step-1-content');
  if (!content) return;
  clearChildren(content);

  const wrapper = createResultContainer('error');

  const text = document.createElement('p');
  text.className = 'result-text';
  text.textContent = 'Claude Code not found';
  wrapper.appendChild(text);

  const errDetail = document.createElement('p');
  errDetail.className = 'result-detail muted';
  errDetail.textContent = String(err);
  wrapper.appendChild(errDetail);

  const downloadP = document.createElement('p');
  downloadP.className = 'result-detail';
  const downloadLink = document.createElement('a');
  downloadLink.href = '#';
  downloadLink.id = 'download-link';
  downloadLink.textContent = 'Download from claude.ai/download';
  downloadLink.addEventListener('click', async (e) => {
    e.preventDefault();
    try {
      window.open('https://claude.ai/download', '_blank');
    } catch {
      window.open('https://claude.ai/download', '_blank');
    }
  });
  downloadP.appendChild(downloadLink);
  wrapper.appendChild(downloadP);

  const manualDiv = document.createElement('div');
  manualDiv.className = 'manual-path-input';

  const label = document.createElement('label');
  label.setAttribute('for', 'claude-path-input');
  label.textContent = 'Or enter the path manually:';
  manualDiv.appendChild(label);

  const input = document.createElement('input');
  input.type = 'text';
  input.id = 'claude-path-input';
  input.placeholder = '/usr/local/bin/claude';
  manualDiv.appendChild(input);

  const continueBtn = document.createElement('button');
  continueBtn.id = 'manual-continue-btn';
  continueBtn.className = 'btn-primary';
  continueBtn.textContent = 'Continue';
  continueBtn.addEventListener('click', () => {
    const manualPath = input.value.trim();
    if (manualPath) {
      startStep2(manualPath);
    }
  });
  manualDiv.appendChild(continueBtn);

  wrapper.appendChild(manualDiv);
  content.appendChild(wrapper);
}

function showStep2UI() {
  const step1 = document.getElementById('step-1-content');
  const step2 = document.getElementById('step-2-content');
  if (step1) step1.style.display = 'none';
  if (step2) step2.style.display = 'block';
}

function showStep2Success() {
  const content = document.getElementById('step-2-content');
  if (!content) return;

  const log = document.getElementById('bootstrap-log');
  clearChildren(content);

  const wrapper = createResultContainer('success');

  const text = document.createElement('p');
  text.className = 'result-text';
  text.textContent = 'Software of You is ready!';
  wrapper.appendChild(text);

  content.appendChild(wrapper);

  if (log) {
    content.appendChild(log);
  }

  const launchBtn = document.createElement('button');
  launchBtn.id = 'launch-btn';
  launchBtn.className = 'btn-primary';
  launchBtn.textContent = 'Launch';
  launchBtn.addEventListener('click', () => {
    completeOnboarding(currentClaudePath);
  });
  content.appendChild(launchBtn);
}

function showStep2Error(err) {
  const content = document.getElementById('step-2-content');
  if (!content) return;

  const log = document.getElementById('bootstrap-log');
  clearChildren(content);

  const wrapper = createResultContainer('error');

  const text = document.createElement('p');
  text.className = 'result-text';
  text.textContent = 'Setup encountered an error';
  wrapper.appendChild(text);

  const errDetail = document.createElement('p');
  errDetail.className = 'result-detail muted';
  errDetail.textContent = String(err);
  wrapper.appendChild(errDetail);

  content.appendChild(wrapper);

  if (log) {
    content.appendChild(log);
  }

  const actions = document.createElement('div');
  actions.className = 'onboarding-actions';

  const retryBtn = document.createElement('button');
  retryBtn.id = 'try-again-btn';
  retryBtn.className = 'btn-primary';
  retryBtn.textContent = 'Try Again';
  retryBtn.addEventListener('click', () => {
    startStep2(currentClaudePath);
  });
  actions.appendChild(retryBtn);

  const helpLink = document.createElement('a');
  helpLink.href = '#';
  helpLink.id = 'get-help-link';
  helpLink.className = 'help-link';
  helpLink.textContent = 'Get Help';
  helpLink.addEventListener('click', (e) => {
    e.preventDefault();
    try {
      window.open('https://softwareofyou.com/help', '_blank');
    } catch {
      window.open('https://softwareofyou.com/help', '_blank');
    }
  });
  actions.appendChild(helpLink);

  content.appendChild(actions);
}

async function runStep1() {
  document.getElementById('onboarding-title').textContent = 'Finding Claude Code...';
  setStepActive(1);

  try {
    const claudePath = await invoke('run_onboarding_step1');
    currentClaudePath = claudePath;
    showStep1Success(claudePath);
    setTimeout(() => startStep2(claudePath), 800);
  } catch (err) {
    showStep1Error(err);
  }
}

async function startStep2(claudePath) {
  currentClaudePath = claudePath;

  document.getElementById('onboarding-title').textContent = 'Setting up Software of You...';
  setStepActive(2);
  showStep2UI();

  const unlisten = await listen('bootstrap-output', (event) => {
    appendBootstrapLine(event.payload);
    if (event.payload.startsWith('ready|')) {
      showStep2Success();
      unlisten();
    }
  });

  try {
    await invoke('run_onboarding_step2', { claudePath });
  } catch (err) {
    showStep2Error(err);
    unlisten();
  }
}

async function completeOnboarding(claudePath) {
  const config = await invoke('get_config');
  config.onboarding_complete = true;
  config.claude_path = claudePath;
  await invoke('save_config', { config });
  document.getElementById('onboarding-screen').style.display = 'none';

  // Start the PTY now that onboarding is complete
  await startPtyAfterOnboarding();
}

async function initOnboarding() {
  try {
    const status = await invoke('check_setup');
    if (status.onboarding_complete === true) {
      const screen = document.getElementById('onboarding-screen');
      if (screen) screen.style.display = 'none';
      return;
    }
  } catch (err) {
    console.error('Failed to check setup status:', err);
  }

  const screen = document.getElementById('onboarding-screen');
  if (screen) screen.style.display = 'flex';

  runStep1();
}

document.addEventListener('DOMContentLoaded', initOnboarding);
