/* global app */
const FIELDS = [
  ['rank', 'Rang & Division'],
  ['mmr', 'MMR'],
  ['mmrChange', 'MMR (Dernier match)'],
  ['mmrSession', 'MMR (Session)'],
  ['gamemode', 'Mode de jeu'],
  ['wins', 'Victoires'],
  ['losses', 'Défaites'],
  ['streak', 'Streak'],
  ['winrate', 'Winrate'],
  ['games', 'Parties'],
  ['best', 'Meilleure streak'],
  ['goals', 'Buts'],
  ['assists', 'Passes'],
  ['saves', 'Arrêts'],
  ['shots', 'Tirs'],
  ['demos', 'Démos'],
  ['deaths', 'Morts'],
];

const RANK_FILES = {
  0: 'unranked.png',
  1: 'bronze-1.png', 2: 'bronze-2.png', 3: 'bronze-3.png',
  4: 'silver-1.png', 5: 'silver-2.png', 6: 'silver-3.png',
  7: 'gold-1.png', 8: 'gold-2.png', 9: 'gold-3.png',
  10: 'platinum-1.png', 11: 'platinum-2.png', 12: 'platinum-3.png',
  13: 'diamond-1.png', 14: 'diamond-2.png', 15: 'diamond-3.png',
  16: 'champion-1.png', 17: 'champion-2.png', 18: 'champion-3.png',
  19: 'grand-champion-1.png', 20: 'grand-champion-2.png', 21: 'grand-champion-3.png',
  22: 'supersonic-legend.png',
};

function getRankImg(tier, size = 36) {
  const file = RANK_FILES[Number(tier)] || 'unranked.png';
  return `<img src="ranks/${file}" alt="Rank" width="${size}" height="${size}" style="object-fit: contain;">`;
}

const $ = (id) => document.getElementById(id);
let S = null;          // état complet reçu du processus principal
let api = null;        // état de la Stats API (fichiers .ini)
let formReady = false;

// ---------------------------------------------------------------------------
// Utilitaires
// ---------------------------------------------------------------------------

const fmtStreak = (n) => (n > 0 ? `+${n}` : String(n));

function calculateRankFrontend(mmr) {
  mmr = Number(mmr) || 0;
  if (mmr <= 0) return { name: 'Unranked', div: 1, tier: 0 };
  const brackets = [
    ['Bronze I', 0, 175, 1],
    ['Bronze II', 175, 235, 2],
    ['Bronze III', 235, 295, 3],
    ['Silver I', 295, 355, 4],
    ['Silver II', 355, 415, 5],
    ['Silver III', 415, 475, 6],
    ['Gold I', 475, 535, 7],
    ['Gold II', 535, 595, 8],
    ['Gold III', 595, 655, 9],
    ['Platinum I', 655, 735, 10],
    ['Platinum II', 735, 815, 11],
    ['Platinum III', 815, 895, 12],
    ['Diamond I', 895, 995, 13],
    ['Diamond II', 995, 1095, 14],
    ['Diamond III', 1095, 1195, 15],
    ['Champion I', 1195, 1315, 16],
    ['Champion II', 1315, 1435, 17],
    ['Champion III', 1435, 1555, 18],
    ['Grand Champion I', 1555, 1715, 19],
    ['Grand Champion II', 1715, 1875, 20],
    ['Grand Champion III', 1875, 2035, 21],
  ];
  if (mmr >= 2035) return { name: 'Supersonic Legend', div: 1, tier: 22 };
  for (const [name, min, max, tier] of brackets) {
    if (mmr < max) {
      const width = (max - min) / 4;
      const div = 1 + Math.min(3, Math.max(0, Math.floor((mmr - min) / width)));
      return { name, div, tier };
    }
  }
  return { name: 'Supersonic Legend', div: 1, tier: 22 };
}

let savedTimer;
function flashSaved(text = 'Enregistré') {
  $('saved').textContent = text;
  clearTimeout(savedTimer);
  savedTimer = setTimeout(() => { $('saved').textContent = ''; }, 1600);
}

async function save(patch) {
  S = await app.save(patch);
  renderForm();
  renderSession();
  flashSaved();
}

function setPressed(container, value) {
  for (const b of container.querySelectorAll('button')) {
    b.setAttribute('aria-pressed', String(b.dataset.value === String(value) || b.dataset.pos === value));
  }
}

// ---------------------------------------------------------------------------
// Colonne session
// ---------------------------------------------------------------------------

function renderSession() {
  const s = S.session;

  const status = $('status');
  status.classList.toggle('connected', S.connected);
  status.querySelector('span').textContent = S.inMatch
    ? 'Match en cours'
    : S.connected ? 'Connecté à Rocket League' : 'En attente de Rocket League';

  // Bloc Rang & MMR
  if ($('sideRankImg')) $('sideRankImg').innerHTML = getRankImg(s.rankTier, 36);
  $('sideRankName').textContent = s.rankName || 'Unranked';
  $('sideRankDiv').textContent = `Division ${s.division || 1}`;
  $('sideMmr').textContent = Number(s.currentMmr || 0).toFixed(1);
  const delta = Number(s.sessionMmrChange || 0);
  $('sideMmrChange').textContent = (delta >= 0 ? '+' : '') + delta.toFixed(1);
  $('sideMmrChange').style.color = delta > 0 ? 'var(--good)' : delta < 0 ? 'var(--orange)' : 'var(--muted)';

  $('streak').textContent = fmtStreak(s.streak);
  $('streakBox').classList.toggle('up', s.streak > 0);
  $('streakBox').classList.toggle('down', s.streak < 0);
  $('wins').textContent = s.wins;
  $('losses').textContent = s.losses;
  $('winrate').textContent = s.games ? `${Math.round((100 * s.wins) / s.games)}%` : '–';

  const recent = $('recent');
  recent.innerHTML = '';
  for (const r of s.recent.slice(-15)) {
    const el = document.createElement('span');
    el.className = r.won ? 'w' : r.abandoned ? 'a' : 'l';
    el.title = `${r.won ? 'Victoire' : r.abandoned ? 'Abandon' : 'Défaite'} - ${r.playlist} (${r.mmrChange >= 0 ? '+' : ''}${r.mmrChange.toFixed(1)} MMR)`;
    recent.appendChild(el);
  }

  const t = s.totals;
  const rows = [
    ['Parties', s.games],
    ['MMR session', (delta >= 0 ? '+' : '') + delta.toFixed(1)],
    ['Buts', t.goals],
    ['Passes décisives', t.assists],
    ['Arrêts', t.saves],
    ['Tirs', t.shots],
    ['Démolitions', t.demos],
    ['Morts', t.deaths],
    ['Meilleure streak', fmtStreak(s.bestStreak)],
    ['Pire série', fmtStreak(s.worstStreak)],
  ];
  $('totals').innerHTML = '';
  for (const [label, value] of rows) {
    const dt = document.createElement('dt');
    const dd = document.createElement('dd');
    dt.textContent = label;
    dd.textContent = value;
    $('totals').append(dt, dd);
  }

  const per = $('perPlaylist');
  per.innerHTML = '';
  for (const pl of Object.values(s.playlists)) {
    const row = document.createElement('div');
    const name = document.createElement('span');
    const record = document.createElement('em');
    const streak = document.createElement('b');
    name.textContent = pl.name;
    record.textContent = `${pl.wins}V – ${pl.losses}D`;
    streak.textContent = fmtStreak(pl.streak);
    streak.style.color = pl.streak > 0 ? 'var(--blue)' : pl.streak < 0 ? 'var(--orange)' : '';
    row.append(name, record, streak);
    per.appendChild(row);
  }

  const since = new Date(s.startedAt);
  $('since').textContent = `Session commencée le ${since.toLocaleDateString('fr-FR')} à ${since.toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' })}`;

  const p = S.settings.player;
  $('detected').textContent = p.detectedName
    ? `Détecté automatiquement : ${p.detectedName}`
    : 'Pas encore détecté : lance une partie, ton joueur sera reconnu tout seul.';
}

// ---------------------------------------------------------------------------
// Formulaire
// ---------------------------------------------------------------------------

function renderForm() {
  const o = S.settings.overlay;
  const c = S.settings.counting;
  const active = document.activeElement;
  const setValue = (el, value) => { if (el !== active) el.value = value; };

  $('overlayEnabled').checked = !!o.enabled;
  setPressed($('theme'), o.theme || 'rocketstats');

  for (const b of $('fields').querySelectorAll('button')) {
    b.setAttribute('aria-pressed', String(o.fields.includes(b.dataset.key)));
  }
  setPressed($('position'), o.position);
  setValue($('offsetX'), o.offsetX);
  setValue($('offsetY'), o.offsetY);
  setValue($('scale'), o.scale);
  $('scaleOut').textContent = `${Math.round(o.scale * 100)} %`;
  setValue($('opacity'), o.opacity != null ? o.opacity : 0.92);
  $('opacityOut').textContent = `${Math.round((o.opacity != null ? o.opacity : 0.92) * 100)} %`;
  setPressed($('layout'), o.layout);

  setValue($('startingMmr'), o.startingMmr || 855);
  setValue($('mmrDelta'), o.mmrDelta || 9);
  updateRankPreview(Number($('startingMmr').value));

  setPressed($('countMode'), c.mode);
  $('customPlaylists').hidden = c.mode !== 'custom';
  renderPlaylistChecklist();
  $('onlineOnly').checked = !!c.onlineOnly;
  $('countLeave').checked = !!c.countLeaveAsLoss;
  $('resetOnStart').checked = !!c.resetOnStart;

  setPressed($('rate'), S.settings.statsApi.packetSendRate);

  const pl = S.settings.player || {};
  setPressed($('playerPlatform'), pl.platform || '');
  const hasPlatform = !!pl.platform;
  $('customName').disabled = !hasPlatform;
  $('customName').placeholder = hasPlatform ? 'Entrez votre pseudo exact...' : 'Cochez d\'abord Epic ou Steam ci-dessus';
  setValue($('customName'), pl.customName || '');
  setValue($('playerMmrInput'), Number(S.session.currentMmr || 855).toFixed(0));

  // Mise à jour de la carte de profil joueur en direct
  if ($('ppRankImg')) $('ppRankImg').innerHTML = getRankImg(S.session.rankTier, 48);
  if ($('ppPseudo')) $('ppPseudo').textContent = pl.customName || 'Joueur Rocket League';
  if ($('ppRankTitle')) $('ppRankTitle').textContent = `${S.session.rankName || 'Unranked'} Div. ${S.session.division || 1}`;
  if ($('ppMmrVal')) $('ppMmrVal').textContent = Number(S.session.currentMmr || 0).toFixed(1);
  if ($('ppPlatBadge')) $('ppPlatBadge').textContent = pl.platform ? (pl.platform === 'epic' ? 'Epic Games' : 'Steam') : 'Détection auto';

  $('obsUrl').textContent = S.obsUrl;
  setValue($('serverPort'), S.settings.server.port);
  $('serverError').hidden = !S.serverError;
  $('serverError').textContent = S.serverError || '';
}

function updateRankPreview(mmr) {
  const rk = calculateRankFrontend(mmr);
  $('rankPreview').textContent = `Rang calculé pour ce MMR : ${rk.name} Div. ${rk.div}`;
}

function renderPlaylistChecklist() {
  const box = $('customPlaylists');
  const selected = S.settings.counting.playlists;
  const ids = new Set(Object.keys(S.playlists).map(Number));
  for (const pl of Object.values(S.session.playlists)) if (pl.id != null) ids.add(pl.id);
  box.innerHTML = '';
  for (const id of [...ids].sort((a, b) => a - b)) {
    const label = document.createElement('label');
    const input = document.createElement('input');
    input.type = 'checkbox';
    input.checked = selected.includes(id);
    input.addEventListener('change', () => {
      const next = input.checked ? [...selected, id] : selected.filter((x) => x !== id);
      save({ counting: { playlists: next } });
    });
    const text = document.createElement('span');
    text.textContent = S.playlists[id] || `Playlist ${id}`;
    label.append(input, text);
    box.appendChild(label);
  }
}

function renderApi(result) {
  if (result) api = result;
  if (!api) return;

  const state = $('apiState');
  state.classList.toggle('on', api.enabled);
  if (!api.valid) state.textContent = 'Dossier de Rocket League introuvable';
  else if (api.enabled) {
    const rate = Math.max(...api.files.filter((f) => f.exists).map((f) => f.packetSendRate));
    state.textContent = `Activée (${rate} envois par seconde)`;
  } else state.textContent = 'Désactivée';

  const dir = $('installDir');
  dir.textContent = api.installDir || 'Aucun dossier sélectionné';
  dir.classList.toggle('missing', !api.valid);

  $('enableApi').textContent = api.enabled ? 'Réappliquer le réglage' : 'Activer la Stats API';
  $('apiBanner').hidden = api.enabled;

  const msg = $('apiMsg');
  msg.className = 'message';
  msg.textContent = '';
  if (result && result.error) { msg.classList.add('error'); msg.textContent = result.error; }
  else if (result && result.success) { msg.classList.add('ok'); msg.textContent = result.success; }
  else if (result && result.notFound) { msg.classList.add('error'); msg.textContent = "Rocket League n'a pas été trouvé automatiquement. Utilise « Choisir le dossier… »."; }
  else if (api.rlRunning && !api.enabled) { msg.classList.add('error'); msg.textContent = 'Rocket League est ouvert : ferme-le avant d\'activer la Stats API.'; }
}

// ---------------------------------------------------------------------------
// Événements
// ---------------------------------------------------------------------------

function bind() {
  // Puces des stats affichées
  for (const [key, label] of FIELDS) {
    const b = document.createElement('button');
    b.type = 'button';
    b.dataset.key = key;
    b.textContent = label;
    b.addEventListener('click', () => {
      const current = S.settings.overlay.fields;
      const next = current.includes(key)
        ? current.filter((k) => k !== key)
        : FIELDS.map(([k]) => k).filter((k) => k === key || current.includes(k));
      save({ overlay: { fields: next } });
    });
    $('fields').appendChild(b);
  }

  $('overlayEnabled').addEventListener('change', (e) => save({ overlay: { enabled: e.target.checked } }));

  $('theme').addEventListener('click', (e) => {
    const v = e.target.closest('button')?.dataset.value;
    if (v) save({ overlay: { theme: v } });
  });

  $('position').addEventListener('click', (e) => {
    const pos = e.target.closest('button')?.dataset.pos;
    if (pos) save({ overlay: { position: pos } });
  });
  for (const id of ['offsetX', 'offsetY']) {
    $(id).addEventListener('change', (e) => save({ overlay: { [id]: Number(e.target.value) || 0 } }));
  }
  $('scale').addEventListener('input', (e) => { $('scaleOut').textContent = `${Math.round(e.target.value * 100)} %`; });
  $('scale').addEventListener('change', (e) => save({ overlay: { scale: Number(e.target.value) } }));

  $('opacity').addEventListener('input', (e) => { $('opacityOut').textContent = `${Math.round(e.target.value * 100)} %`; });
  $('opacity').addEventListener('change', (e) => save({ overlay: { opacity: Number(e.target.value) } }));

  $('layout').addEventListener('click', (e) => {
    const v = e.target.closest('button')?.dataset.value;
    if (v) save({ overlay: { layout: v } });
  });

  $('startingMmr').addEventListener('input', (e) => updateRankPreview(Number(e.target.value)));
  $('applyMmr').addEventListener('click', async () => {
    const mmr = Number($('startingMmr').value);
    if (!isNaN(mmr) && mmr >= 0) {
      S = await app.setMmr(mmr);
      renderSession();
      renderForm();
      flashSaved('MMR mis à jour');
    }
  });

  $('savePlayerMmrBtn').addEventListener('click', async () => {
    const mmr = Number($('playerMmrInput').value);
    if (!isNaN(mmr) && mmr >= 0) {
      S = await app.setMmr(mmr);
      renderSession();
      renderForm();
      flashSaved('MMR & Profil mis à jour');
    }
  });

  $('playerMmrInput').addEventListener('input', (e) => {
    const val = Number(e.target.value);
    if (!isNaN(val) && val >= 0) {
      const rk = calculateRankFrontend(val);
      if ($('ppRankTitle')) $('ppRankTitle').textContent = `${rk.name} Div. ${rk.div}`;
      if ($('ppMmrVal')) $('ppMmrVal').textContent = val.toFixed(1);
      if ($('ppRankImg')) $('ppRankImg').innerHTML = getRankImg(rk.tier, 48);
    }
  });

  $('customName').addEventListener('input', (e) => {
    if ($('ppPseudo')) $('ppPseudo').textContent = e.target.value.trim() || 'Joueur Rocket League';
  });

  $('mmrDelta').addEventListener('change', (e) => {
    const delta = Number(e.target.value);
    if (!isNaN(delta) && delta > 0) save({ overlay: { mmrDelta: delta } });
  });

  $('countMode').addEventListener('click', (e) => {
    const v = e.target.closest('button')?.dataset.value;
    if (v) save({ counting: { mode: v } });
  });
  $('onlineOnly').addEventListener('change', (e) => save({ counting: { onlineOnly: e.target.checked } }));
  $('countLeave').addEventListener('change', (e) => save({ counting: { countLeaveAsLoss: e.target.checked } }));
  $('resetOnStart').addEventListener('change', (e) => save({ counting: { resetOnStart: e.target.checked } }));

  $('rate').addEventListener('click', (e) => {
    const v = e.target.closest('button')?.dataset.value;
    if (v) save({ statsApi: { packetSendRate: Number(v) } });
  });
  $('detect').addEventListener('click', async () => renderApi(await app.detectInstall()));
  $('choose').addEventListener('click', async () => renderApi(await app.chooseInstall()));
  $('enableApi').addEventListener('click', async () => {
    const btn = $('enableApi');
    btn.disabled = true;
    try { renderApi(await app.enableStatsApi()); } finally { btn.disabled = false; }
  });
  $('goToApi').addEventListener('click', () => $('sec-api').scrollIntoView({ behavior: 'smooth' }));

  $('playerPlatform').addEventListener('click', (e) => {
    const v = e.target.closest('button')?.dataset.value;
    if (v !== undefined) {
      const hasPlat = !!v;
      $('customName').disabled = !hasPlat;
      $('customName').placeholder = hasPlat ? 'Entrez votre pseudo exact...' : 'Cochez d\'abord Epic ou Steam ci-dessus';
      if (!hasPlat) {
        $('customName').value = '';
        save({ player: { platform: '', customName: '' } });
      } else {
        save({ player: { platform: v } });
        $('customName').focus();
      }
    }
  });

  $('customName').addEventListener('change', (e) => {
    save({ player: { customName: e.target.value.trim() } });
  });

  $('copyUrl').addEventListener('click', async () => { await app.copy(S.obsUrl); flashSaved('Copié'); });
  $('openObs').addEventListener('click', () => app.openObsFolder());
  $('serverPort').addEventListener('change', (e) => {
    const port = Number(e.target.value);
    if (port >= 1024 && port <= 65535) save({ server: { port } });
  });

  // Réinitialisation en deux temps pour éviter les clics accidentels
  let confirmTimer;
  $('reset').addEventListener('click', async () => {
    const btn = $('reset');
    if (!btn.classList.contains('confirm')) {
      btn.classList.add('confirm');
      btn.textContent = 'Cliquer à nouveau pour confirmer';
      confirmTimer = setTimeout(() => {
        btn.classList.remove('confirm');
        btn.textContent = 'Réinitialiser la session';
      }, 3000);
      return;
    }
    clearTimeout(confirmTimer);
    btn.classList.remove('confirm');
    btn.textContent = 'Réinitialiser la session';
    S = await app.reset();
    renderSession();
    flashSaved('Session remise à zéro');
  });

  $('quit').addEventListener('click', () => app.quit());

  app.onState((state) => {
    S = state;
    renderSession();
    if (formReady) renderForm();
  });

  window.addEventListener('focus', async () => renderApi(await app.statsApiStatus()));
}

(async function init() {
  bind();
  S = await app.getState();
  renderSession();
  renderForm();
  formReady = true;
  renderApi(await app.statsApiStatus());
})();
