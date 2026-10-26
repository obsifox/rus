/* Rus — talks to the Rust backend. English UI, left sidebar. */
'use strict';

const $ = (s, r = document) => r.querySelector(s);
const $$ = (s, r = document) => Array.from(r.querySelectorAll(s));
const esc = (s) =>
  String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));

const S = {
  tab: 'home',
  settings: null,
  system: {},
  uuid: '',
  installed: [],
  running: { running: false },
  horizon: {},
  versions: [],
  latest: {},
  patches: [],
  mods: [],
  skinsList: { skins: [], capes: [] },
  filters: { channel: 'all', q: '', installedOnly: false, limit: 40 },
  selected: null,
  job: null,
  jobTimer: null,
  horizonTimer: null,
  sandbox: null,
};

const PRESETS = [
  { id: 'preset:steve', name: 'Steve' },
  { id: 'preset:alex', name: 'Alex' },
];
const CAPES = [
  { id: 'preset:none', name: 'None' },
  { id: 'preset:rus', name: 'Rus' },
];

async function api(path, body) {
  const opt =
    body === undefined
      ? {}
      : { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) };
  const r = await fetch(path, opt);
  const text = await r.text();
  let data = {};
  try { data = text ? JSON.parse(text) : {}; } catch { data = { error: text.slice(0, 200) }; }
  if (!r.ok) throw new Error(data.error || `HTTP ${r.status}`);
  return data;
}

function toast(msg, kind = '') {
  const el = document.createElement('div');
  el.className = `toast ${kind}`;
  el.textContent = msg;
  $('#toasts').appendChild(el);
  setTimeout(() => el.remove(), 4200);
}

const fmtBytes = (n) => {
  if (!n) return '0 B';
  const u = ['B', 'KB', 'MB', 'GB'];
  let i = 0;
  while (n >= 1024 && i < u.length - 1) { n /= 1024; i++; }
  return `${n.toFixed(n < 10 && i > 0 ? 1 : 0)} ${u[i]}`;
};
const fmtDate = (iso) => {
  if (!iso) return '—';
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso.slice(0, 10) : d.toISOString().slice(0, 10);
};

const TEX = new Map();
function tex(id) {
  if (!id || id === 'preset:none') return Promise.resolve(null);
  if (TEX.has(id)) return TEX.get(id);
  const p = new Promise((resolve) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = () => resolve(null);
    img.src = `/api/texture?id=${encodeURIComponent(id)}`;
  });
  TEX.set(id, p);
  return p;
}

function drawHead(cv, img) {
  cv.width = 64;
  cv.height = 64;
  const ctx = cv.getContext('2d');
  ctx.imageSmoothingEnabled = false;
  ctx.fillStyle = '#111';
  ctx.fillRect(0, 0, 64, 64);
  if (!img) return;
  ctx.drawImage(img, 8, 8, 8, 8, 0, 0, 64, 64);
  ctx.drawImage(img, 40, 8, 8, 8, 0, 0, 64, 64);
}

const DOLL_PARTS = [
  [44, 20, 4, 12, 0, 8, 4, 12],
  [36, 52, 4, 12, 12, 8, 4, 12],
  [20, 20, 8, 12, 4, 8, 8, 12],
  [4, 20, 4, 12, 4, 20, 4, 12],
  [20, 52, 4, 12, 8, 20, 4, 12],
  [8, 8, 8, 8, 4, 0, 8, 8],
];
const DOLL_OVER = [
  [44, 36, 4, 12, 0, 8, 4, 12],
  [52, 52, 4, 12, 12, 8, 4, 12],
  [20, 36, 8, 12, 4, 8, 8, 12],
  [0, 36, 4, 12, 4, 20, 4, 12],
  [0, 52, 4, 12, 8, 20, 4, 12],
  [40, 8, 8, 8, 4, 0, 8, 8],
];

function drawDoll(cv, skinImg, capeImg, slim) {
  const px = 6;
  cv.width = 16 * px;
  cv.height = 32 * px;
  const ctx = cv.getContext('2d');
  ctx.imageSmoothingEnabled = false;
  ctx.clearRect(0, 0, cv.width, cv.height);
  if (capeImg) {
    // Official cape back face is at (12, 1), 10×16 — that is the visible side.
    ctx.drawImage(capeImg, 12, 1, 10, 16, 2 * px, 7 * px, 12 * px, 19 * px);
  }
  if (!skinImg) return;
  const draw = ([sx, sy, w, h, dx, dy, dw, dh], arm) => {
    let x = dx, ww = dw;
    if (slim && arm) { ww = 3; x = dx === 0 ? 1 : 12; }
    ctx.drawImage(skinImg, sx, sy, w, h, x * px, dy * px, ww * px, dh * px);
  };
  DOLL_PARTS.forEach((p, i) => draw(p, i < 2));
  DOLL_OVER.forEach((p, i) => draw(p, i < 2));
}

function skinId() {
  const id = S.settings && S.settings.skin;
  if (!id || id === 'preset:steve' || id === 'file:preset_steve.png') return 'preset:steve';
  if (id === 'preset:alex' || id === 'file:preset_alex.png') return 'preset:alex';
  return id;
}
function capeId() {
  const id = S.settings && S.settings.cape;
  if (!id || id === 'preset:none' || id === 'preset:') return 'preset:none';
  if (id === 'preset:rus' || id === 'file:preset_rus.png') return 'preset:rus';
  return id;
}

async function paintAvatars() {
  if (!S.settings) return;
  const head = await tex(skinId());
  $$('canvas[data-head]').forEach((cv) => drawHead(cv, head));
  const cape = await tex(capeId());
  for (const cv of $$('canvas[data-doll]')) {
    drawDoll(cv, head, cape, S.settings.skin_model === 'slim');
  }
  $$('canvas[data-skin-id]').forEach(async (cv) => drawHead(cv, await tex(cv.dataset.skinId)));
  $$('canvas[data-cape-id]').forEach(async (cv) => {
    const img = await tex(cv.dataset.capeId);
    const ctx = cv.getContext('2d');
    cv.width = 64; cv.height = 64;
    ctx.imageSmoothingEnabled = false;
    ctx.fillStyle = '#0a0a0a';
    ctx.fillRect(0, 0, 64, 64);
    if (!img) {
      ctx.strokeStyle = '#2c2c2c';
      ctx.strokeRect(16.5, 8.5, 30, 46);
      return;
    }
    ctx.drawImage(img, 12, 1, 10, 16, 16, 8, 32, 48);
  });
}

function engine() {
  return (S.settings && S.settings.engine) || 'vanilla';
}

function paintChrome() {
  if (!S.settings) return;
  $('#sideName').textContent = S.settings.username || 'Player';
  const sel = S.selected || S.settings.last_version || '—';
  $('#selectedVersion').textContent = engine() === 'horizon' ? '26.3' : sel;
  $$('.engine-btn').forEach((b) => b.classList.toggle('is-on', b.dataset.engine === engine()));
  const hz = S.horizon || {};
  $('#engineNote').textContent = engine() === 'horizon'
    ? (hz.prepared ? 'Horizon 26.3 · sandboxed' : 'Rust client · sandboxed')
    : 'Official client · sandboxed';
  const running = S.running && S.running.running;
  const box = S.sandbox || {};
  const seal = $('#envSeal');
  if (seal) {
    seal.classList.toggle('is-on', !!box.sealed);
    const label = seal.querySelector('span');
    if (label) label.textContent = box.label || 'Sandbox';
  }
  const envTitle = $('#envTitle');
  if (envTitle) {
    envTitle.textContent = running
      ? `Session · ${S.running.version || 'client'}`
      : 'Sandbox idle';
  }
  const sealLine = $('#sealLine');
  if (sealLine) sealLine.textContent = box.label || 'Sandbox';
  const root = $('#envRoot');
  if (root) root.textContent = box.root || (S.system && S.system.data_dir) || '';
  const policy = $('#envPolicy');
  if (policy) {
    policy.textContent = box.hides_home
      ? 'Home hidden · network shared · dies with Rus'
      : 'Private profile · network shared';
  }
  paintSession();
  const btn = $('#playBtn');
  btn.classList.toggle('is-stop', !!running);
  if (running) {
    btn.textContent = 'Stop';
    $('#launchStatus').textContent = `Running ${S.running.version || ''}`.trim();
  } else if (hz.phase && hz.phase !== 'idle' && hz.phase !== 'ready' && hz.phase !== 'error') {
    btn.textContent = 'Play Horizon';
    $('#launchStatus').textContent = hz.message || 'Preparing Horizon…';
  } else if (engine() === 'horizon') {
    btn.textContent = 'Play Horizon';
    $('#launchStatus').textContent = hz.error || (hz.prepared ? 'Ready' : 'Will prepare on first launch');
  } else {
    const inst = S.installed.some((x) => x.id === S.selected);
    btn.textContent = S.selected && !inst ? 'Install' : 'Play';
    $('#launchStatus').textContent = S.selected && !inst ? 'Not installed' : 'Ready';
  }
  paintAvatars();
}

async function saveSettings(patch = {}) {
  const next = { ...S.settings, ...patch };
  const saved = await api('/api/settings', next);
  S.settings = saved.settings || next;
  paintChrome();
  return S.settings;
}

function pageHead(title, sub) {
  return `<header class="page-head">
    <div>
      <div class="page-kicker">Rus</div>
      <h1 class="page-title">${esc(title)}</h1>
    </div>
    <p class="page-sub">${sub || ''}</p>
  </header>`;
}

function paintSession() {
  const title = $('#sessionState');
  const note = $('#sessionNote');
  const floor = $('#floor');
  const running = S.running && S.running.running;
  if (floor) floor.classList.toggle('is-live', !!running);
  if (title) title.textContent = running ? `Live · ${S.running.version || 'client'}` : 'Idle';
  if (note) {
    note.textContent = running
      ? `pid ${S.running.pid || '—'} · drawing in the sandbox window`
      : 'Nothing is running. Play starts the client in this sandbox, not in a browser.';
  }
}

function renderHome() {
  const st = S.settings;
  const latest = S.latest || {};
  const inst = S.installed.length;
  const mods = (S.mods || []).filter((m) => m.enabled).length;
  const java = (S.system && S.system.java) ? 'Java ready' : 'Java missing';
  const hz = S.horizon || {};
  const showSnap = st.show_snapshots && latest.snapshot;
  const box = S.sandbox || {};
  const host = box.hostname || '—';
  const files = box.hides_home ? 'Not mounted' : 'Private profile';
  $('#view').innerHTML = `
    <section class="stage">
      <div class="floor" id="floor">
        <div class="floor-copy">
          <div class="page-kicker">Sandbox session</div>
          <h2 id="sessionState">Idle</h2>
          <p id="sessionNote">Nothing is running. Play starts the client in this sandbox, not in a browser.</p>
        </div>
        <div class="doll-wrap"><canvas data-doll width="96" height="192"></canvas></div>
      </div>
      <aside class="panel policy">
        <h3>Sandbox</h3>
        <p class="hint">${esc(box.detail || 'Checking the jail…')}</p>
        <div class="pol"><span>Isolation</span><b>${esc(box.label || 'Sandbox')}</b></div>
        <div class="pol"><span>Hostname</span><b>${esc(host)}</b></div>
        <div class="pol"><span>Your files</span><b>${esc(files)}</b></div>
        <div class="pol"><span>Network</span><b>Shared</b></div>
        <div class="pol"><span>Window</span><b>Own window</b></div>
        <div class="pol"><span>Lifetime</span><b>${box.dies_with_rus ? 'Dies with Rus' : 'Private profile'}</b></div>
      </aside>
    </section>
    <div class="chips" style="margin: 14px 0">
      <span class="chip ${S.system.java ? 'ok' : 'warn'}">${esc(java)}</span>
      <span class="chip">Release <strong>${esc(latest.release || '—')}</strong></span>
      ${showSnap ? `<span class="chip warn">Snapshot <strong>${esc(latest.snapshot)}</strong></span>` : ''}
      <span class="chip ${hz.installed ? 'ok' : ''}">Horizon ${hz.prepared ? 'ready' : hz.installed ? 'installed' : 'not installed'}</span>
    </div>
    <section class="stats">
      <div class="stat"><span>Memory</span><b>${st.ram_mb || 0}<small style="font-size:14px;color:var(--muted)"> MB</small></b></div>
      <div class="stat"><span>Mods on</span><b>${mods}</b></div>
      <div class="stat"><span>Installed</span><b>${inst}</b></div>
    </section>
    <section class="grid-2">
      <div class="panel">
        <h3>Patches</h3>
        <p class="hint">Each layer can be turned off before an install starts.</p>
        ${(S.patches || []).map((p) => `<div class="row"><div><span class="tag">${esc(p.key)}</span><span class="name">${esc(p.title)}</span><div class="meta">${esc(p.detail)}</div></div></div>`).join('') || '<div class="empty">Waiting for the version list.</div>'}
      </div>
      <div class="panel">
        <h3>Latest</h3>
        <p class="hint">From Mojang. Snapshots stay hidden until you enable them.</p>
        ${latest.release ? `<div class="row"><div><span class="tag">Release</span><span class="name">${esc(latest.release)}</span></div><button class="btn" data-install="${esc(latest.release)}">Install</button></div>` : ''}
        ${showSnap ? `<div class="row"><div><span class="tag snap">Snapshot</span><span class="name">${esc(latest.snapshot)}</span></div><button class="btn" data-install="${esc(latest.snapshot)}">Install</button></div>` : `<div class="row"><div class="meta">Snapshots are off. Enable them in Settings.</div></div>`}
      </div>
    </section>`;
  $$('[data-install]').forEach((b) => (b.onclick = () => openInstall(b.dataset.install)));
  paintAvatars();
  paintSession();
}

function filteredVersions() {
  let v = S.versions.slice();
  const q = S.filters.q.trim().toLowerCase();
  if (q) v = v.filter((x) => x.id.toLowerCase().includes(q));
  if (S.filters.channel !== 'all') v = v.filter((x) => x.channel === S.filters.channel);
  if (S.filters.installedOnly) v = v.filter((x) => x.installed);
  return v;
}

function renderVersions() {
  const st = S.settings || {};
  const all = filteredVersions();
  const shown = all.slice(0, S.filters.limit);
  const chips = [
    ['all', 'All'],
    ['release', 'Release'],
  ];
  if (st.show_snapshots) chips.push(['snapshot', 'Snapshot']);
  if (st.show_previews) chips.push(['preview', 'Preview']);
  $('#view').innerHTML = `
    ${pageHead('Versions', 'Releases always. Snapshots and previews only if you turn them on in Settings.')}
    ${st.show_snapshots ? '' : '<div class="note">Snapshots are hidden. Turn on “Show snapshots” in Settings to list them.</div>'}
    <div class="toolbar">
      <input class="search" id="q" placeholder="Search versions" value="${esc(S.filters.q)}">
      ${chips.map(([id, label]) => `<button class="fbtn ${S.filters.channel === id ? 'is-on' : ''}" data-ch="${id}">${label}</button>`).join('')}
      <button class="fbtn ${S.filters.installedOnly ? 'is-on' : ''}" id="onlyInst">Installed</button>
    </div>
    <div class="vmeta" style="margin-bottom:8px">${shown.length} of ${all.length}</div>
    <div class="vlist">
      ${shown.map((v) => `
        <div class="vrow ${S.selected === v.id ? 'is-sel' : ''}" data-pick="${esc(v.id)}">
          <div>
            <div class="vid">${esc(v.id)}</div>
            <div class="vmeta">${esc(fmtDate(v.released))}${v.installed ? ' · installed' : ''}</div>
          </div>
          <span class="tag ${v.channel === 'snapshot' ? 'snap' : v.channel === 'preview' ? 'pre' : ''}">${esc(v.channel)}</span>
          <div class="vactions">
            <button class="btn" data-install="${esc(v.id)}">${v.installed ? 'Repair' : 'Install'}</button>
            ${v.installed ? `<button class="btn danger" data-del="${esc(v.id)}">Remove</button>` : ''}
          </div>
        </div>`).join('') || '<div class="empty">No versions match.</div>'}
    </div>
    ${all.length > shown.length ? `<div style="margin-top:12px"><button class="btn" id="more">Show more</button></div>` : ''}`;
  const q = $('#q');
  q.oninput = () => { S.filters.q = q.value; S.filters.limit = 40; renderVersions(); q.focus(); };
  $$('[data-ch]').forEach((b) => (b.onclick = () => { S.filters.channel = b.dataset.ch; S.filters.limit = 40; renderVersions(); }));
  const oi = $('#onlyInst');
  if (oi) oi.onclick = () => { S.filters.installedOnly = !S.filters.installedOnly; renderVersions(); };
  $$('[data-pick]').forEach((row) => (row.onclick = (e) => {
    if (e.target.closest('button')) return;
    S.selected = row.dataset.pick;
    saveSettings({ last_version: S.selected }).catch(() => {});
    paintChrome();
    $$('.vrow').forEach((r) => r.classList.toggle('is-sel', r.dataset.pick === S.selected));
  }));
  $$('[data-install]').forEach((b) => (b.onclick = () => openInstall(b.dataset.install)));
  $$('[data-del]').forEach((b) => (b.onclick = async () => {
    if (!confirm(`Remove ${b.dataset.del}?`)) return;
    try {
      await api('/api/uninstall', { version: b.dataset.del });
      toast(`Removed ${b.dataset.del}`);
      await refreshState();
      await refreshVersions();
      render();
    } catch (e) { toast(e.message, 'err'); }
  }));
  const more = $('#more');
  if (more) more.onclick = () => { S.filters.limit += 40; renderVersions(); };
}

function renderMods() {
  const mods = S.mods || [];
  $('#view').innerHTML = `
    ${pageHead('Mods', 'Drop a .jar. Enabled mods are copied into the selected version when you install or play.')}
    <div class="drop" id="drop"><strong>Drop mod jars here</strong><div>or click to browse</div><input id="modFile" type="file" accept=".jar" multiple hidden></div>
    ${mods.length ? mods.map((m) => `
      <div class="mod">
        <div>
          <div class="name">${esc(m.name)}</div>
          <div class="meta">${fmtBytes(m.size)}${m.enabled ? '' : ' · off'}</div>
        </div>
        <button class="switch ${m.enabled ? 'on' : ''}" data-toggle="${esc(m.name)}" title="Enable"><i></i></button>
        <button class="btn danger" data-rm="${esc(m.name)}">Remove</button>
      </div>`).join('') : '<div class="empty">No mods yet.</div>'}
    <div style="margin-top:12px"><button class="btn" id="stage">Stage onto selected version</button></div>`;
  const drop = $('#drop');
  const input = $('#modFile');
  drop.onclick = () => input.click();
  drop.ondragover = (e) => { e.preventDefault(); drop.classList.add('over'); };
  drop.ondragleave = () => drop.classList.remove('over');
  drop.ondrop = (e) => { e.preventDefault(); drop.classList.remove('over'); addMods(e.dataTransfer.files); };
  input.onchange = () => addMods(input.files);
  $$('[data-toggle]').forEach((b) => (b.onclick = async () => {
    try { await api('/api/mods', { action: 'toggle', name: b.dataset.toggle }); await refreshMods(); renderMods(); }
    catch (e) { toast(e.message, 'err'); }
  }));
  $$('[data-rm]').forEach((b) => (b.onclick = async () => {
    try { await api('/api/mods', { action: 'remove', name: b.dataset.rm }); await refreshMods(); renderMods(); }
    catch (e) { toast(e.message, 'err'); }
  }));
  $('#stage').onclick = async () => {
    const v = S.selected;
    if (!v) return toast('Select a version first', 'warn');
    try {
      const r = await api('/api/mods', { action: 'stage', version: v });
      toast(`Staged ${r.staged} mod${r.staged === 1 ? '' : 's'} onto ${v}`);
    } catch (e) { toast(e.message, 'err'); }
  };
}

function readFile(file) {
  return new Promise((resolve, reject) => {
    const r = new FileReader();
    r.onload = () => resolve(r.result);
    r.onerror = () => reject(r.error);
    r.readAsDataURL(file);
  });
}
async function addMods(files) {
  for (const f of files || []) {
    if (!f.name.endsWith('.jar')) { toast(`${f.name} is not a jar`, 'warn'); continue; }
    try {
      const data = await readFile(f);
      await api('/api/mods', { action: 'add', name: f.name, data });
      toast(`Added ${f.name}`);
    } catch (e) { toast(e.message, 'err'); }
  }
  await refreshMods();
  renderMods();
}

function renderSettings() {
  const st = S.settings;
  const skins = [
    ...PRESETS,
    ...(S.skinsList.skins || []).filter((s) => !String(s.id).includes('preset_')),
  ];
  const capes = [
    ...CAPES,
    ...(S.skinsList.capes || []).filter((c) => !String(c.id).includes('preset_')),
  ];
  $('#view').innerHTML = `
    ${pageHead('Settings', 'Username, Steve by default, the Rus cape, memory, and who can see snapshots.')}
    <div class="set-grid">
      <div class="panel">
        <h3>Account</h3>
        <p class="hint">Offline only. The UUID matches vanilla’s OfflinePlayer name.</p>
        <div class="preview-doll"><canvas data-doll></canvas></div>
        <div class="field">
          <label>Username</label>
          <input id="user" maxlength="16" value="${esc(st.username)}" autocomplete="off">
        </div>
        <div class="meta" style="margin-bottom:12px">${esc(S.uuid || '')}</div>
        <div class="field">
          <label>Model</label>
          <select id="model">
            <option value="classic" ${st.skin_model !== 'slim' ? 'selected' : ''}>Classic (Steve, 4px arms)</option>
            <option value="slim" ${st.skin_model === 'slim' ? 'selected' : ''}>Slim (Alex, 3px arms)</option>
          </select>
        </div>
        <div class="field"><label>Skin</label>
          <div class="tiles" id="skinTiles">
            ${skins.map((s) => `<button type="button" class="tile ${skinId() === s.id ? 'is-on' : ''}" data-skin="${esc(s.id)}"><canvas data-head data-skin-id="${esc(s.id)}" width="64" height="64"></canvas><div class="cap">${esc(s.name || s.id)}</div></button>`).join('')}
          </div>
          <div style="margin-top:8px"><button class="btn" id="upSkin" type="button">Upload PNG</button></div>
        </div>
        <div class="field"><label>Cape</label>
          <div class="tiles" id="capeTiles">
            ${capes.map((c) => `<button type="button" class="tile ${capeId() === c.id ? 'is-on' : ''}" data-cape="${esc(c.id)}">${c.id === 'preset:none' ? '<div class="swatch"></div>' : `<canvas data-cape-id="${esc(c.id)}" width="64" height="64"></canvas>`}<div class="cap">${esc(c.name || c.id)}</div></button>`).join('')}
          </div>
          <div class="hint" style="margin-top:8px">Rus is the bundled cape, drawn in the real 64×32 layout.</div>
          <button class="btn" id="upCape" type="button">Upload cape PNG</button>
        </div>
      </div>
      <div>
        <div class="panel" style="margin-bottom:12px">
          <h3>Game</h3>
          <div class="field">
            <label>Memory · ${st.ram_mb} MB</label>
            <input id="ram" type="range" min="512" max="${Math.max(2048, (S.system.ram_mb || 8192) - 512)}" step="256" value="${st.ram_mb}">
          </div>
          <div class="field">
            <label>Java path</label>
            <input id="java" placeholder="Empty = automatic" value="${esc(st.java_path || '')}">
          </div>
          <div class="field">
            <label>JVM arguments</label>
            <textarea id="jvm">${esc(st.jvm_args || '')}</textarea>
          </div>
          <div class="field">
            <label>Window</label>
            <div class="inline" style="direction:ltr">
              <input id="w" type="number" min="640" value="${st.width}">
              <span>×</span>
              <input id="h" type="number" min="480" value="${st.height}">
            </div>
          </div>
          <label class="check"><span>Fullscreen<small>Start the game fullscreen</small></span><input id="fs" type="checkbox" ${st.fullscreen ? 'checked' : ''}></label>
          <label class="check"><span>Demo mode<small>Passed to the game as --demo</small></span><input id="demo" type="checkbox" ${st.demo ? 'checked' : ''}></label>
        </div>
        <div class="panel">
          <h3>Versions</h3>
          <p class="hint">Off by default. Nothing snapshot or preview is listed until you enable it.</p>
          <label class="check"><span>Show snapshots<small>Weekly snapshots, 26 and newer</small></span><input id="snaps" type="checkbox" ${st.show_snapshots ? 'checked' : ''}></label>
          <label class="check"><span>Show previews<small>Pre-releases and release candidates, 26 and newer</small></span><input id="prevs" type="checkbox" ${st.show_previews ? 'checked' : ''}></label>
          <div class="field" style="margin-top:12px">
            <label>Game folder</label>
            <input id="gdir" placeholder="Empty = launcher data folder" value="${esc(st.game_dir || '')}">
          </div>
          <div class="inline">
            <button class="btn" data-open="data" type="button">Data folder</button>
            <button class="btn" data-open="game" type="button">Game folder</button>
            <button class="btn" data-open="logs" type="button">Logs</button>
          </div>
        </div>
      </div>
    </div>`;
  $('#user').onchange = async () => {
    const name = $('#user').value.trim();
    if (!/^[A-Za-z0-9_]{3,16}$/.test(name)) return toast('Use 3–16 letters, numbers or _', 'warn');
    await saveSettings({ username: name });
    const stt = await api('/api/state');
    S.uuid = stt.uuid;
    toast('Name saved');
    renderSettings();
  };
  $('#model').onchange = () => saveSettings({ skin_model: $('#model').value });
  $('#ram').oninput = (e) => { const lab = e.target.previousElementSibling; if (lab) lab.textContent = `Memory · ${e.target.value} MB`; };
  $('#ram').onchange = () => saveSettings({ ram_mb: Number($('#ram').value) });
  $('#java').onchange = () => saveSettings({ java_path: $('#java').value.trim() });
  $('#jvm').onchange = () => saveSettings({ jvm_args: $('#jvm').value });
  $('#w').onchange = () => saveSettings({ width: Number($('#w').value) || 1280 });
  $('#h').onchange = () => saveSettings({ height: Number($('#h').value) || 720 });
  $('#fs').onchange = () => saveSettings({ fullscreen: $('#fs').checked });
  $('#demo').onchange = () => saveSettings({ demo: $('#demo').checked });
  $('#gdir').onchange = () => saveSettings({ game_dir: $('#gdir').value.trim() });
  $('#snaps').onchange = async () => {
    await saveSettings({ show_snapshots: $('#snaps').checked });
    await refreshVersions();
    toast($('#snaps').checked ? 'Snapshots are listed' : 'Snapshots hidden');
  };
  $('#prevs').onchange = async () => {
    await saveSettings({ show_previews: $('#prevs').checked });
    await refreshVersions();
  };
  $$('[data-skin]').forEach((b) => (b.onclick = async () => {
    await saveSettings({ skin: b.dataset.skin });
    renderSettings();
  }));
  $$('[data-cape]').forEach((b) => (b.onclick = async () => {
    await saveSettings({ cape: b.dataset.cape });
    renderSettings();
  }));
  $('#upSkin').onclick = () => pickImage('skin');
  $('#upCape').onclick = () => pickImage('cape');
  $$('[data-open]').forEach((b) => (b.onclick = () => api('/api/open', { path: b.dataset.open }).catch((e) => toast(e.message, 'err'))));
  paintAvatars();
}

function pickImage(kind) {
  const input = document.createElement('input');
  input.type = 'file';
  input.accept = 'image/png';
  input.onchange = async () => {
    const f = input.files && input.files[0];
    if (!f) return;
    try {
      const data = await readFile(f);
      const r = await api('/api/skins', { kind, name: f.name.replace(/\.png$/i, ''), data });
      S.skinsList = await api('/api/skins');
      TEX.delete(r.id);
      await saveSettings(kind === 'cape' ? { cape: r.id } : { skin: r.id });
      renderSettings();
    } catch (e) { toast(e.message, 'err'); }
  };
  input.click();
}

function openInstall(version) {
  S.selected = version;
  saveSettings({ last_version: version }).catch(() => {});
  const patches = (S.patches || []).map((p) => ({ ...p, enabled: true, status: 'pending', note: '' }));
  const ov = $('#overlay');
  const draw = () => {
    ov.classList.remove('hidden');
    ov.innerHTML = `<div class="dialog">
      <h2>Install ${esc(version)}</h2>
      <p class="sub">Turn a patch off before it starts. Downloads are split into parts — a dropped connection or a bad part resumes from that part, not from zero.</p>
      ${patches.map((p) => `<label class="patch">
        <input type="checkbox" data-pk="${esc(p.key)}" ${p.enabled ? 'checked' : ''} ${S.job ? 'disabled' : ''}>
        <div><b>${esc(p.title)}</b><div class="d">${esc(p.detail)}</div><div class="d">${esc(p.note || '')}</div></div>
        <div class="st ${esc(p.status)}">${esc(p.status)}</div>
      </label>`).join('')}
      <div class="bar"><i id="bar" style="width:${Math.round((S.job && S.job.progress || 0) * 100)}%"></i></div>
      <div class="meta" id="jobMsg">${esc((S.job && S.job.message) || 'Ready')}</div>
      <div class="log">${((S.job && S.job.logs) || []).slice(-6).map(esc).join('<br>')}</div>
      <div class="dialog-actions">
        <button class="btn" id="dlgClose" type="button">Close</button>
        <div style="display:flex;gap:8px">
          ${S.job && S.job.state === 'running' ? '<button class="btn danger" id="dlgCancel" type="button">Cancel</button>' : ''}
          <button class="btn accent" id="dlgGo" type="button" ${S.job && S.job.state === 'running' ? 'disabled' : ''}>Install</button>
        </div>
      </div>
    </div>`;
    $$('[data-pk]').forEach((c) => (c.onchange = () => {
      const p = patches.find((x) => x.key === c.dataset.pk);
      if (p) p.enabled = c.checked;
    }));
    $('#dlgClose').onclick = () => { stopJob(); ov.classList.add('hidden'); ov.innerHTML = ''; };
    const cancel = $('#dlgCancel');
    if (cancel) cancel.onclick = async () => { if (S.job) await api('/api/cancel', { job: S.job.id }).catch(() => {}); };
    $('#dlgGo').onclick = () => beginInstall(version, patches, draw);
  };
  draw();
}

function stopJob() {
  if (S.jobTimer) clearInterval(S.jobTimer);
  S.jobTimer = null;
}

async function beginInstall(version, patches, draw) {
  const sel = {};
  patches.forEach((p) => { sel[p.key] = p.enabled; });
  try {
    const r = await api('/api/install', { version, patches: sel });
    S.job = { id: r.job, state: 'running', progress: 0, message: 'Starting…', logs: [] };
    draw();
    S.jobTimer = setInterval(async () => {
      try {
        const j = await api(`/api/job?id=${encodeURIComponent(r.job)}`);
        S.job = j;
        (j.patches || []).forEach((jp) => {
          const p = patches.find((x) => x.key === jp.key);
          if (p) Object.assign(p, jp);
        });
        draw();
        if (j.state !== 'running') {
          stopJob();
          toast(j.state === 'done' ? `${version} installed` : (j.error || j.message), j.state === 'done' ? '' : 'err');
          await refreshState();
          await refreshVersions();
          if (S.tab !== 'version') paintChrome();
        }
      } catch (e) { toast(e.message, 'err'); }
    }, 700);
  } catch (e) { toast(e.message, 'err'); }
}

async function doPlay() {
  if (S.running && S.running.running) {
    try { await api('/api/kill'); toast('Stopped'); setTimeout(pollRunning, 300); }
    catch (e) { toast(e.message, 'err'); }
    return;
  }
  if (engine() === 'horizon') {
    try {
      const r = await api('/api/play', { version: '26.3', engine: 'horizon' });
      toast(r.message || 'Horizon is starting');
      pollHorizon();
    } catch (e) {
      if (String(e.message).includes('Install Minecraft 26.3')) {
        toast('Install 26.3 first — Horizon uses that copy', 'warn');
        openInstall('26.3');
      } else toast(e.message, 'err');
    }
    return;
  }
  const v = S.selected || (S.settings && S.settings.last_version);
  if (!v) return toast('Select a version', 'warn');
  if (!S.installed.some((x) => x.id === v)) {
    toast(`${v} is not installed`);
    return openInstall(v);
  }
  try {
    const r = await api('/api/play', { version: v, engine: 'vanilla' });
    toast(`Started ${v} in the sandbox`);
    S.selected = v;
    pollRunning();
    paintChrome();
  } catch (e) { toast(e.message, 'err'); }
}

async function pollRunning() {
  try { S.running = await api('/api/running'); } catch { S.running = { running: false }; }
  paintChrome();
  setTimeout(pollRunning, S.running.running ? 1000 : 4000);
}

function pollHorizon() {
  if (S.horizonTimer) clearInterval(S.horizonTimer);
  const tick = async () => {
    try { S.horizon = await api('/api/horizon'); } catch { return; }
    paintChrome();
    const phase = S.horizon.phase;
    if (phase === 'error') {
      clearInterval(S.horizonTimer);
      toast(S.horizon.error || S.horizon.message, 'err');
    } else if (phase === 'ready' || (S.running && S.running.running && S.running.engine === 'horizon')) {
      clearInterval(S.horizonTimer);
      pollRunning();
    }
  };
  tick();
  S.horizonTimer = setInterval(tick, 1000);
}

function switchTab(tab) {
  S.tab = tab;
  $$('.nav-btn').forEach((b) => b.classList.toggle('is-active', b.dataset.tab === tab));
  render();
}
function render() {
  if (!S.settings) return;
  if (S.tab === 'home') renderHome();
  else if (S.tab === 'version') renderVersions();
  else if (S.tab === 'mods') renderMods();
  else renderSettings();
}

$$('.nav-btn').forEach((b) => (b.onclick = () => switchTab(b.dataset.tab)));
$$('.engine-btn').forEach((b) => (b.onclick = async () => {
  await saveSettings({ engine: b.dataset.engine });
  toast(b.dataset.engine === 'horizon' ? 'Play will launch Horizon 26.3' : 'Play will launch Vanilla');
  render();
}));
$('#playBtn').onclick = () => doPlay();
$('#accountChip').onclick = () => switchTab('settings');
document.addEventListener('keydown', (e) => {
  if (e.ctrlKey && e.key >= '1' && e.key <= '4') {
    e.preventDefault();
    switchTab(['home', 'version', 'mods', 'settings'][Number(e.key) - 1]);
  }
  if (e.key === 'Escape') {
    const ov = $('#overlay');
    if (!ov.classList.contains('hidden')) { stopJob(); ov.classList.add('hidden'); ov.innerHTML = ''; }
  }
});

async function refreshState() {
  const st = await api('/api/state');
  S.settings = st.settings;
  S.system = st.system || {};
  S.uuid = st.uuid || '';
  S.installed = st.installed || [];
  S.running = st.running || { running: false };
  S.horizon = st.horizon || {};
  if (!S.selected) S.selected = S.settings.last_version || (S.installed[0] && S.installed[0].id) || null;
  paintChrome();
}
async function refreshVersions() {
  try {
    const r = await api('/api/versions');
    S.versions = r.versions || [];
    S.latest = r.latest || {};
    if (r.patches) S.patches = r.patches;
    if (!S.selected && S.latest.release) S.selected = S.latest.release;
  } catch (e) { toast(`Version list failed: ${e.message}`, 'err'); }
}
async function refreshMods() {
  const r = await api('/api/mods');
  S.mods = r.mods || [];
}

async function boot() {
  const veil = document.createElement('div');
  veil.className = 'loading-veil';
  veil.textContent = 'RUS';
  $('#app').appendChild(veil);
  try {
    await refreshState();
    if (!S.settings.skin || S.settings.skin === 'preset:') S.settings.skin = 'preset:steve';
    S.skinsList = await api('/api/skins');
    await Promise.all([refreshVersions(), refreshMods()]);
    if (!S.selected && S.latest.release) S.selected = S.latest.release;
    paintChrome();
    render();
    pollRunning();
    if (S.horizon.phase && !['idle', 'ready', 'error'].includes(S.horizon.phase)) pollHorizon();
  } catch (e) {
    toast(`Cannot reach Rus: ${e.message}`, 'err');
  } finally {
    veil.remove();
  }
}
boot();
