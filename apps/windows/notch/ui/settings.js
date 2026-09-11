const invoke = window.__TAURI__.core.invoke;
const names = {claude:'Claude',cursor:'Cursor',codex:'Codex',antigravity:'Antigravity',glm:'GLM',grok:'Grok',opencode:'OpenCode',commandcode:'Command Code',copilot:'GitHub Copilot','ollama-local':'Ollama Local',ollama:'Ollama Cloud','gemini-api':'Gemini API'};
const status = document.querySelector('#status');
let settings;

function checkbox(label, checked, change) {
  const wrapper = document.createElement('label');
  const input = document.createElement('input');
  input.type = 'checkbox'; input.checked = checked;
  input.addEventListener('change', () => change(input.checked));
  wrapper.append(input, document.createTextNode(label));
  return wrapper;
}

function membership(field, id, enabled) {
  settings[field] = settings[field].filter(existing => existing !== id);
  if (enabled) settings[field].push(id);
}

function moveProvider(index, direction) {
  const destination = index + direction;
  if (destination < 0 || destination >= settings.provider_order.length) return;
  [settings.provider_order[index], settings.provider_order[destination]] = [settings.provider_order[destination], settings.provider_order[index]];
  renderProviders();
}

function renderProviders() {
  const list = document.querySelector('#provider-list');
  list.replaceChildren();
  settings.provider_order.forEach((id, index) => {
    const row = document.createElement('div'); row.className = 'provider';
    row.append(checkbox(names[id], !settings.disabled_providers.includes(id), enabled => membership('disabled_providers', id, !enabled)));
    row.append(checkbox('Mute', settings.muted_providers.includes(id), muted => membership('muted_providers', id, muted)));
    for (const [label, direction] of [['↑', -1], ['↓', 1]]) {
      const button = document.createElement('button'); button.textContent = label;
      button.setAttribute('aria-label', `Move ${names[id]} ${direction < 0 ? 'up' : 'down'}`);
      button.disabled = index + direction < 0 || index + direction >= settings.provider_order.length;
      button.onclick = () => moveProvider(index, direction); row.append(button);
    }
    list.append(row);
  });
}

// All displays draws a notch on every monitor, so picking one monitor or following the active window no longer applies
function syncDisplayControls() {
  const everyDisplay = document.querySelector('[data-setting=scope]').value === 'all_displays';
  document.querySelector('#display').disabled = everyDisplay;
  document.querySelector('[data-setting=follow_focus]').disabled = everyDisplay;
}

function renderSettings() {
  document.querySelectorAll('[data-setting]').forEach(input => {
    const setting = settings[input.dataset.setting];
    if (input.type === 'checkbox') input.checked = setting; else input.value = setting ?? '';
  });
  document.querySelector('#display').value = settings.display || '';
  syncDisplayControls();
  renderProviders();
}

function inputValue(input) {
  if (input.type === 'checkbox') return input.checked;
  if (input.hasAttribute('data-optional') && input.value === '') return null;
  return ['range', 'number'].includes(input.type) ? Number(input.value) : input.value;
}

async function saveSettings() {
  const invalid = [...document.querySelectorAll('input[type=number]')].find(input => !input.checkValidity());
  if (invalid) { invalid.reportValidity(); throw new Error('Check the highlighted setting.'); }
  document.querySelectorAll('[data-setting]').forEach(input => {
    settings[input.dataset.setting] = inputValue(input);
  });
  settings.display = document.querySelector('#display').value || null;
  settings.onboarding_complete = true;
  await invoke('save_settings', {settings});
  status.textContent = 'Settings saved.';
}

function action(selector, operation) {
  document.querySelector(selector).onclick = async () => {
    try { await operation(); } catch (error) { status.textContent = String(error); }
  };
}

async function initialize() {
  Object.assign(names, await invoke('get_profile_names'));
  settings = await invoke('get_settings');
  const displays = await invoke('get_displays');
  for (const display of displays) {
    const option = document.createElement('option'); option.value = display; option.textContent = display;
    document.querySelector('#display').append(option);
  }
  renderSettings();
}

document.querySelectorAll('[data-pane]').forEach(button => {
  button.onclick = () => {
    document.querySelectorAll('section').forEach(section => { section.hidden = section.id !== button.dataset.pane; });
    document.querySelectorAll('[data-pane]').forEach(tab => tab.setAttribute('aria-current', String(tab === button)));
  };
});
action('#save', saveSettings);
action('#save-key', async () => {
  const input = document.querySelector('#ollama-key');
  await invoke('save_ollama_key', {key: input.value}); input.value = '';
  status.textContent = 'Key saved in Windows Credential Manager.';
});
action('#delete-key', async () => { await invoke('delete_ollama_key'); status.textContent = 'Saved key removed.'; });
action('#reset-display', async () => {
  settings.display = null; settings.edge = 'right'; settings.notch_y = 0.5; settings.scale = 1;
  settings.scope = 'main_display'; settings.visibility = 'on_hover';
  renderSettings(); await saveSettings();
});
document.querySelector('[data-setting=scope]').addEventListener('change', syncDisplayControls);
initialize().catch(error => { status.textContent = String(error); });

function showUpdate(update) {
  const preview = update.phase === 'development';
  document.querySelector('#update-status').textContent = update.error || (preview ? 'Development build. Public updates are disabled.'
    : update.version ? 'Notch ' + update.version + ' · ' + update.phase : update.phase === 'current' ? 'Notch is up to date.' : 'No update check completed yet.');
  document.querySelector('#release-notes').textContent = update.notes || '';
  document.querySelector('#check-update').disabled = preview;
  document.querySelector('[data-setting=automatic_checks]').disabled = preview;
  document.querySelector('#download-update').hidden = update.phase !== 'available';
  document.querySelector('#install-update').hidden = update.phase !== 'ready';
  document.querySelector('#later-update').hidden = !['available','ready'].includes(update.phase);
}
action('#check-update', async () => showUpdate(await invoke('check_update')));
action('#download-update', async () => { await invoke('download_update'); });
action('#install-update', async () => {
  if (window.confirm('Restart Notch and install the verified update now?')) await invoke('install_update');
});
action('#later-update', async () => {
  document.querySelector('#download-update').hidden = true; document.querySelector('#install-update').hidden = true;
  document.querySelector('#later-update').hidden = true;
});
window.__TAURI__.event.listen('update-status', event => showUpdate(event.payload)).catch(error => { status.textContent = String(error); });
invoke('update_status').then(showUpdate).catch(error => { status.textContent = String(error); });
