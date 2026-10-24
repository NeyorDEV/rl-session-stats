// Pas de console noire au lancement en version finale.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod rl_config;
mod session;
mod settings;
mod stats_client;
mod web_server;

#[cfg(test)]
mod tests;

use serde_json::{json, Value};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent, Wry,
};
use tauri_plugin_dialog::DialogExt;

use session::{calculate_rank, now_ms, CountRules, Effects, Session, Tracker, PLAYLISTS, RANKED};
use settings::SettingsStore;
use stats_client::ClientEvent;
use web_server::WebServer;

const OFFSCREEN: f64 = -32000.0;

// ---------------------------------------------------------------------------
// État partagé
// ---------------------------------------------------------------------------

struct Core {
    store: SettingsStore,
    tracker: Tracker,
    connected: bool,
    overlay_size: (f64, f64),
    obs_dir: PathBuf,
    session_file: PathBuf,
    history_file: PathBuf,
    log_file: PathBuf,
    documents_dir: Option<PathBuf>,
}

impl Core {
    fn rules(&self) -> CountRules {
        let c = &self.store.data.counting;
        let o = &self.store.data.overlay;
        CountRules {
            online_only: c.online_only,
            mode: c.mode.clone(),
            playlists: c.playlists.clone(),
            count_leave_as_loss: c.count_leave_as_loss,
            mmr_delta: if o.mmr_delta > 0.0 { o.mmr_delta } else { 9.0 },
            focused_playlist: c.focused_playlist.clone(),
        }
    }

    fn identity(&self) -> Option<String> {
        let p = &self.store.data.player;
        if !p.custom_name.is_empty() {
            Some(format!("NAME:{}|{}", p.platform, p.custom_name))
        } else {
            let id = if p.primary_id.is_empty() { &p.detected_id } else { &p.primary_id };
            (!id.is_empty()).then(|| id.clone())
        }
    }

    fn log(&self, message: &str) {
        if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(&self.log_file) {
            let _ = writeln!(f, "{message}");
        }
    }

    fn overlay_state(&self) -> Value {
        json!({
            "connected": self.connected,
            "inMatch": self.tracker.in_match(),
            "session": self.tracker.session,
            "display": self.store.data.overlay,
        })
    }

    fn load_history(&self) -> Vec<Value> {
        fs::read_to_string(&self.history_file)
            .ok()
            .and_then(|t| serde_json::from_str::<Vec<Value>>(&t).ok())
            .unwrap_or_default()
    }

    fn archive_session(&self, s: &Session) {
        if s.games == 0 {
            return;
        }
        let mut list = self.load_history();
        let started_at = s.started_at;
        let ended_at = now_ms();
        let record = json!({
            "id": format!("{started_at}"),
            "startedAt": started_at,
            "endedAt": ended_at,
            "games": s.games,
            "wins": s.wins,
            "losses": s.losses,
            "streak": s.streak,
            "bestStreak": s.best_streak,
            "sessionMmrChange": s.session_mmr_change,
            "currentMmr": s.current_mmr,
            "startingMmr": s.current_mmr - s.session_mmr_change,
            "rankName": s.rank_name,
            "division": s.division,
            "rankTier": s.rank_tier,
            "focusedPlaylist": s.focused_playlist,
            "recent": s.recent
        });

        if let Some(pos) = list.iter().position(|item| item["id"].as_str() == Some(&format!("{started_at}"))) {
            list[pos] = record;
        } else {
            list.insert(0, record);
        }

        if list.len() > 60 {
            list.truncate(60);
        }

        if let Ok(text) = serde_json::to_string_pretty(&list) {
            let _ = fs::write(&self.history_file, text);
        }
    }

    fn delete_history_item(&self, id: &str) {
        let mut list = self.load_history();
        list.retain(|item| item["id"].as_str() != Some(id));
        if let Ok(text) = serde_json::to_string_pretty(&list) {
            let _ = fs::write(&self.history_file, text);
        }
    }

    fn full_state(&self, server_error: Option<String>) -> Value {
        let playlists: serde_json::Map<String, Value> =
            PLAYLISTS.iter().map(|(id, name)| (id.to_string(), json!(name))).collect();
        let history = self.load_history();
        json!({
            "settings": self.store.data,
            "session": self.tracker.session,
            "history": history,
            "connected": self.connected,
            "inMatch": self.tracker.in_match(),
            "playlists": playlists,
            "ranked": RANKED,
            "obsUrl": format!("http://localhost:{}/overlay", self.store.data.server.port),
            "obsDir": self.obs_dir,
            "serverError": server_error,
            "shortcut": "Ctrl+Maj+O",
        })
    }

    fn tooltip(&self) -> String {
        let s = &self.tracker.session;
        format!("RL Session Stats - {}V / {}D - streak {} - MMR {:.1}", s.wins, s.losses, fmt_streak(s.streak), s.current_mmr)
    }

    /// Sauvegarde la session et écrit les fichiers texte pour OBS.
    fn persist_session(&self) {
        let s = &self.tracker.session;
        if let Ok(text) = serde_json::to_string_pretty(s) {
            let _ = fs::write(&self.session_file, text);
        }
        if s.games > 0 {
            self.archive_session(s);
        }
        let _ = fs::create_dir_all(&self.obs_dir);
        let winrate = if s.games > 0 { format!("{}%", (100 * s.wins + s.games / 2) / s.games) } else { "-".into() };
        let mmr_str = format!("{:.1}", s.current_mmr);
        let mmr_change_str = format!("{:+.1}", s.last_match_mmr_change);
        let mmr_session_str = format!("{:+.1}", s.session_mmr_change);
        let rank_full = format!("{} Div. {}", s.rank_name, s.division);
        let files = [
            ("games", s.games.to_string()),
            ("wins", s.wins.to_string()),
            ("losses", s.losses.to_string()),
            ("streak", fmt_streak(s.streak)),
            ("winrate", winrate),
            ("best", fmt_streak(s.best_streak)),
            ("goals", s.totals.goals.to_string()),
            ("assists", s.totals.assists.to_string()),
            ("saves", s.totals.saves.to_string()),
            ("shots", s.totals.shots.to_string()),
            ("demos", s.totals.demos.to_string()),
            ("deaths", s.totals.deaths.to_string()),
            ("mmr", mmr_str),
            ("mmr_change", mmr_change_str),
            ("mmr_session", mmr_session_str),
            ("rank", s.rank_name.clone()),
            ("division", format!("Div. {}", s.division)),
            ("rank_full", rank_full),
            ("mode", s.focused_playlist.clone()),
        ];
        for (name, value) in files {
            let _ = fs::write(self.obs_dir.join(format!("{name}.txt")), value);
        }
    }
}

fn fmt_streak(n: i64) -> String {
    if n > 0 { format!("+{n}") } else { n.to_string() }
}

struct Ctx {
    core: Arc<Mutex<Core>>,
    rl_port: Arc<AtomicU16>,
    web: WebServer,
    toggle_item: Mutex<Option<CheckMenuItem<Wry>>>,
    preview_overlay: Arc<AtomicBool>,
}

// ---------------------------------------------------------------------------
// Diffusion de l'état (fenêtres, OBS, icône)
// ---------------------------------------------------------------------------

fn publish(app: &AppHandle) {
    let ctx = app.state::<Ctx>();
    let (full, overlay, tooltip, enabled) = {
        let c = ctx.core.lock().unwrap();
        (c.full_state(ctx.web.error()), c.overlay_state(), c.tooltip(), c.store.data.overlay.enabled)
    };
    let _ = app.emit("state", &full);
    let _ = app.emit("overlay", &overlay);
    ctx.web.broadcast(&overlay);
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_tooltip(Some(tooltip));
    }
    let item = ctx.toggle_item.lock().unwrap().clone();
    if let Some(item) = item {
        let _ = item.set_checked(enabled);
    }
}

fn after_effects(app: &AppHandle, effects: Effects) {
    let ctx = app.state::<Ctx>();
    {
        let mut c = ctx.core.lock().unwrap();
        for line in &effects.logs {
            c.log(line);
        }
        if let Some((id, name)) = &effects.detected {
            let p = &c.store.data.player;
            if p.primary_id.is_empty() && (p.detected_id != *id || p.detected_name != *name) {
                c.store.update(json!({ "player": { "detectedId": id, "detectedName": name } }));
            }
        }
        if effects.session_changed {
            c.persist_session();
            let auto_sync = c.store.data.player.auto_sync_mmr;
            if auto_sync {
                let app_handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    std::thread::sleep(Duration::from_secs(4));
                    let _ = perform_tracker_sync(app_handle.clone()).await;
                    std::thread::sleep(Duration::from_secs(6));
                    let _ = perform_tracker_sync(app_handle).await;
                });
            }
        }
    }
    if effects.session_changed || effects.status_changed || effects.detected.is_some() {
        publish(app);
    }
}

// ---------------------------------------------------------------------------
// Overlay en jeu
// ---------------------------------------------------------------------------

fn render_overlay_on_screen(app: &AppHandle) {
    let Some(win) = app.get_webview_window("overlay") else { return };
    let (o, (w, h), is_preview) = {
        let ctx = app.state::<Ctx>();
        let c = ctx.core.lock().unwrap();
        (c.store.data.overlay.clone(), c.overlay_size, ctx.preview_overlay.load(Ordering::Relaxed))
    };
    if !o.enabled && !is_preview {
        let _ = win.set_position(LogicalPosition::new(OFFSCREEN, OFFSCREEN));
        let _ = win.set_ignore_cursor_events(true);
        return;
    }
    let Some(monitor) = win.primary_monitor().ok().flatten() else { return };
    let sf = monitor.scale_factor();
    let pos = monitor.position().to_logical::<f64>(sf);
    let size = monitor.size().to_logical::<f64>(sf);
    let (w, h) = (w.max(40.0), h.max(20.0));

    let (x, y) = if o.position == "custom" {
        let cx = o.custom_x.unwrap_or(o.offset_x);
        let cy = o.custom_y.unwrap_or(o.offset_y);
        (pos.x + cx, pos.y + cy)
    } else {
        let (vertical, horizontal) = o.position.split_once('-').unwrap_or(("top", "left"));
        let x = match horizontal {
            "left" => pos.x + o.offset_x,
            "right" => pos.x + size.width - w - o.offset_x,
            _ => pos.x + (size.width - w) / 2.0 + o.offset_x,
        };
        let y = if vertical == "bottom" {
            pos.y + size.height - h - o.offset_y
        } else {
            pos.y + o.offset_y
        };
        (x, y)
    };

    let _ = win.set_size(LogicalSize::new(w, h));
    let _ = win.set_position(LogicalPosition::new(x.round(), y.round()));
    let _ = win.set_always_on_top(true);
    let _ = win.set_ignore_cursor_events(!is_preview);
}

fn hide_overlay(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("overlay") {
        let _ = win.set_position(LogicalPosition::new(OFFSCREEN, OFFSCREEN));
        let _ = win.set_ignore_cursor_events(true);
    }
}

fn apply_overlay(app: &AppHandle) {
    let (enabled, only_in_game, is_preview) = {
        let ctx = app.state::<Ctx>();
        let c = ctx.core.lock().unwrap();
        (c.store.data.overlay.enabled, c.store.data.overlay.only_in_game, ctx.preview_overlay.load(Ordering::Relaxed))
    };
    if is_preview {
        render_overlay_on_screen(app);
        return;
    }
    if !enabled {
        hide_overlay(app);
        return;
    }
    if only_in_game && !rl_config::is_rocket_league_focused() {
        hide_overlay(app);
    } else {
        render_overlay_on_screen(app);
    }
}

fn toggle_overlay(app: &AppHandle) {
    {
        let ctx = app.state::<Ctx>();
        let mut c = ctx.core.lock().unwrap();
        let enabled = !c.store.data.overlay.enabled;
        c.store.update(json!({ "overlay": { "enabled": enabled } }));
    }
    apply_overlay(app);
    publish(app);
}

fn open_settings(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("settings") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}

fn reset_session(app: &AppHandle) {
    {
        let ctx = app.state::<Ctx>();
        let mut c = ctx.core.lock().unwrap();
        if c.tracker.session.games > 0 {
            c.archive_session(&c.tracker.session.clone());
        }
        c.tracker.reset();
        c.persist_session();
        c.log("Session réinitialisée");
    }
    publish(app);
}

// ---------------------------------------------------------------------------
// Commandes appelées par l'interface
// ---------------------------------------------------------------------------

#[tauri::command]
fn get_state(app: AppHandle) -> Value {
    let ctx = app.state::<Ctx>();
    let c = ctx.core.lock().unwrap();
    c.full_state(ctx.web.error())
}

#[tauri::command]
fn save_settings(app: AppHandle, patch: Value) -> Value {
    {
        let ctx = app.state::<Ctx>();
        let mut c = ctx.core.lock().unwrap();
        c.store.update(patch);
        let focus = c.store.data.counting.focused_playlist.clone();
        c.tracker.update_focus(&focus);
        ctx.rl_port.store(c.store.data.stats_api.port, Ordering::SeqCst);
        c.persist_session();
    }
    apply_overlay(&app);
    publish(&app);
    get_state(app)
}

#[tauri::command]
fn reset_session_cmd(app: AppHandle) -> Value {
    reset_session(&app);
    get_state(app)
}

#[tauri::command]
fn set_mmr_cmd(app: AppHandle, mmr: f64) -> Value {
    {
        let ctx = app.state::<Ctx>();
        let mut c = ctx.core.lock().unwrap();
        let focus = c.store.data.counting.focused_playlist.clone();
        c.tracker.set_mmr(mmr, &focus);
        c.store.update(json!({ "overlay": { "startingMmr": mmr } }));
        c.persist_session();
    }
    publish(&app);
    get_state(app)
}

fn stats_api_status(app: &AppHandle) -> Value {
    let (dir, docs) = {
        let ctx = app.state::<Ctx>();
        let c = ctx.core.lock().unwrap();
        (c.store.data.stats_api.install_dir.clone(), c.documents_dir.clone())
    };
    let mut status = rl_config::read_status(&dir, docs.as_deref());
    status["rlRunning"] = json!(rl_config::is_rocket_league_running());
    status
}

#[tauri::command]
async fn statsapi_status(app: AppHandle) -> Value {
    stats_api_status(&app)
}

#[tauri::command]
async fn statsapi_detect(app: AppHandle) -> Value {
    let installs = rl_config::detect_game_installations();
    let target = installs.epic.clone().or(installs.steam.clone());
    if let Some(dir) = target {
        let ctx = app.state::<Ctx>();
        ctx.core.lock().unwrap().store.update(json!({ "statsApi": { "installDir": dir } }));
    }
    publish(&app);
    let mut status = stats_api_status(&app);
    status["notFound"] = json!(installs.epic.is_none() && installs.steam.is_none());
    status
}

#[tauri::command]
async fn statsapi_select_platform(app: AppHandle, platform: String) -> Value {
    let installs = rl_config::detect_game_installations();
    let target = match platform.to_lowercase().as_str() {
        "epic" => installs.epic,
        "steam" => installs.steam,
        _ => None,
    };
    if let Some(dir) = target {
        {
            let ctx = app.state::<Ctx>();
            ctx.core.lock().unwrap().store.update(json!({ "statsApi": { "installDir": dir } }));
        }
        publish(&app);
    }
    stats_api_status(&app)
}

#[tauri::command]
async fn detect_player_now(app: AppHandle) -> Value {
    let docs = app.state::<Ctx>().core.lock().unwrap().documents_dir.clone();
    if let Some(player) = rl_config::detect_player_from_logs(docs.as_deref()) {
        {
            let ctx = app.state::<Ctx>();
            let mut c = ctx.core.lock().unwrap();
            c.store.update(json!({
                "player": {
                    "platform": player.platform,
                    "customName": player.name
                }
            }));
            c.log(&format!("Joueur d\u{e9}tect\u{e9} : {} ({})", player.name, player.platform));
        }
        publish(&app);
        let h = app.clone();
        tauri::async_runtime::spawn(async move {
            let _ = perform_tracker_sync(h).await;
        });
        json!({
            "success": true,
            "platform": player.platform,
            "name": player.name,
            "id": player.id
        })
    } else {
        json!({
            "success": false,
            "error": "Aucun profil trouv\u{e9} dans les fichiers de log (Launch.log). Lancez Rocket League au moins une fois."
        })
    }
}

#[tauri::command]
async fn statsapi_choose(app: AppHandle) -> Value {
    let picked = app
        .dialog()
        .file()
        .set_title("Choisis le dossier d'installation de Rocket League")
        .blocking_pick_folder()
        .and_then(|p| p.into_path().ok());
    let Some(picked) = picked else { return stats_api_status(&app) };

    match rl_config::normalize_install_dir(&picked) {
        Some(dir) => {
            {
                let ctx = app.state::<Ctx>();
                let dir = dir.to_string_lossy().into_owned();
                ctx.core.lock().unwrap().store.update(json!({ "statsApi": { "installDir": dir } }));
            }
            publish(&app);
            stats_api_status(&app)
        }
        None => {
            let mut status = stats_api_status(&app);
            status["error"] = json!("Ce dossier ne contient pas Rocket League (il faut le dossier qui contient « TAGame »).");
            status
        }
    }
}

#[tauri::command]
async fn statsapi_enable(app: AppHandle) -> Value {
    let (dir, docs, rate, port) = {
        let ctx = app.state::<Ctx>();
        let c = ctx.core.lock().unwrap();
        let s = &c.store.data.stats_api;
        (s.install_dir.clone(), c.documents_dir.clone(), s.packet_send_rate, s.port)
    };
    let mut status = stats_api_status(&app);
    if !rl_config::is_valid_install(&dir) {
        status["error"] = json!("Dossier de Rocket League introuvable. Utilise « Détecter » ou « Choisir le dossier ».");
        return status;
    }
    if rl_config::is_rocket_league_running() {
        status["error"] = json!("Ferme complètement Rocket League, puis réessaie : le jeu écrase ce réglage en quittant.");
        return status;
    }
    match rl_config::enable_stats_api(&dir, docs.as_deref(), rate, port) {
        Ok(_) => {
            app.state::<Ctx>().core.lock().unwrap().log("Stats API activée");
            let mut status = stats_api_status(&app);
            status["success"] = json!("Stats API activée. Lance Rocket League : le réglage est lu au démarrage du jeu.");
            status
        }
        Err(err) => {
            status["error"] = json!(if err.kind() == std::io::ErrorKind::PermissionDenied {
                "Windows refuse l'écriture dans le dossier du jeu. Quitte l'appli, fais un clic droit dessus > « Exécuter en tant qu'administrateur », puis réessaie.".to_string()
            } else {
                format!("Impossible de modifier le fichier : {err}")
            });
            status
        }
    }
}

#[tauri::command]
fn open_obs_folder(app: AppHandle) {
    let dir = app.state::<Ctx>().core.lock().unwrap().obs_dir.clone();
    let _ = fs::create_dir_all(&dir);
    #[cfg(windows)]
    let _ = std::process::Command::new("explorer").arg(&dir).spawn();
    #[cfg(not(windows))]
    let _ = std::process::Command::new("xdg-open").arg(&dir).spawn();
}

#[tauri::command]
fn open_browser_url(url: String) {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return;
    }
    #[cfg(windows)]
    let _ = std::process::Command::new("cmd").args(["/C", "start", "", &url]).spawn();
    #[cfg(not(windows))]
    let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
}

fn urlencoding_decode(input: &str) -> String {
    let mut out = Vec::new();
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(b) = u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        if bytes[i] == b'+' {
            out.push(b' ');
        } else {
            out.push(bytes[i]);
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

async fn perform_tracker_sync(app: AppHandle) -> Result<Value, String> {
    let (platform, username, focus) = {
        let ctx = app.state::<Ctx>();
        let c = ctx.core.lock().unwrap();
        let p = &c.store.data.player;
        let mut user = p.custom_name.trim().to_string();
        if user.is_empty() {
            user = p.primary_id.trim().to_string();
        }
        if user.is_empty() {
            user = p.detected_name.trim().to_string();
        }
        let mut plat = p.platform.trim().to_lowercase();
        if plat.is_empty() {
            plat = "epic".to_string();
        }
        (plat, user, c.store.data.counting.focused_playlist.clone())
    };

    if username.is_empty() {
        return Err("Veuillez d'abord renseigner votre pseudo dans l'onglet Joueur.".into());
    }

    let encoded_user: String = username
        .chars()
        .map(|c| match c {
            ' ' => "%20".to_string(),
            '#' => "%23".to_string(),
            _ => c.to_string(),
        })
        .collect();

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    let url_str = format!(
        "https://rocketleague.tracker.network/rocket-league/profile/{}/{}?_nocache={}",
        platform,
        encoded_user,
        timestamp
    );

    let parsed_url = url_str
        .parse::<tauri::Url>()
        .map_err(|e| format!("URL invalide : {e}"))?;

    // Détruire une éventuelle fenêtre existante
    if let Some(old) = app.get_webview_window("tracker_sync") {
        let _ = old.destroy();
    }

    let init_script = r#"
(function() {
  try {
    const s = document.createElement('style');
    s.innerHTML = 'html, body { display: none !important; opacity: 0 !important; visibility: hidden !important; }';
    (document.head || document.documentElement).appendChild(s);
  } catch (e) {}

  let finished = false;
  function report(success, data) {
    if (finished) return;
    finished = true;
    window.location.hash = (success ? 'TRN_SUCCESS=' : 'TRN_ERROR=') + encodeURIComponent(JSON.stringify(data));
  }

  function tryFetch() {
    try {
      const parts = window.location.pathname.split('/');
      const idx = parts.indexOf('profile');
      if (idx !== -1 && parts.length > idx + 2) {
        const plat = parts[idx + 1];
        const user = parts[idx + 2];
        const bust = Date.now();
        fetch('https://api.tracker.gg/api/v2/rocket-league/standard/profile/' + plat + '/' + user + '?_nocache=' + bust, {
          cache: 'no-store',
          headers: {
            'Accept': 'application/json',
            'Cache-Control': 'no-cache, no-store, must-revalidate',
            'Pragma': 'no-cache'
          },
          credentials: 'include'
        })
        .then(r => {
          if (r.ok) return r.json();
          throw new Error('HTTP ' + r.status);
        })
        .then(data => {
          if (data && data.data && data.data.segments) {
            report(true, data.data.segments);
          }
        })
        .catch(() => {});
      }
    } catch (e) {}
  }

  function tryDom() {
    try {
      if (document.body && (document.body.innerText.includes('Player Not Found') || document.body.innerText.includes('We could not find'))) {
        report(false, 'Profil introuvable sur Rocket League Tracker');
      }
    } catch (e) {}
  }

  let attempts = 0;
  const timer = setInterval(() => {
    attempts++;
    tryFetch();
    tryDom();
    if (attempts > 30) clearInterval(timer);
  }, 800);
  setTimeout(tryFetch, 400);
})();
"#;

    let win = WebviewWindowBuilder::new(&app, "tracker_sync", WebviewUrl::External(parsed_url))
        .title("RL Tracker Sync")
        .inner_size(1.0, 1.0)
        .position(OFFSCREEN, OFFSCREEN)
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .skip_taskbar(true)
        .resizable(true)
        .focused(false)
        .visible(false)
        .initialization_script(init_script)
        .build()
        .map_err(|e| format!("Impossible d'ouvrir le navigateur en arrière-plan : {e}"))?;

    let _ = win.hide();

    let mut result_json: Option<Value> = None;
    let mut err_msg: Option<String> = None;

    for tick in 0..50 {
        std::thread::sleep(Duration::from_millis(500));

        if tick % 3 == 0 {
            let _ = win.eval(r#"
                (function() {
                  try {
                    const parts = window.location.pathname.split('/');
                    const idx = parts.indexOf('profile');
                    if (idx !== -1 && parts.length > idx + 2) {
                      const plat = parts[idx + 1];
                      const user = parts[idx + 2];
                      const bust = Date.now();
                      fetch('https://api.tracker.gg/api/v2/rocket-league/standard/profile/' + plat + '/' + user + '?_nocache=' + bust, {
                        cache: 'no-store',
                        headers: {
                          'Accept': 'application/json',
                          'Cache-Control': 'no-cache, no-store, must-revalidate',
                          'Pragma': 'no-cache'
                        },
                        credentials: 'include'
                      })
                      .then(r => r.json())
                      .then(data => {
                        if (data && data.data && data.data.segments) {
                          window.location.hash = 'TRN_SUCCESS=' + encodeURIComponent(JSON.stringify(data.data.segments));
                        }
                      })
                      .catch(() => {});
                    }
                  } catch(e) {}
                })();
            "#);
        }

        if let Ok(u) = win.url() {
            let s = u.as_str();
            if let Some(pos) = s.find("#TRN_SUCCESS=") {
                let payload = &s[pos + "#TRN_SUCCESS=".len()..];
                let decoded = urlencoding_decode(payload);
                if let Ok(val) = serde_json::from_str::<Value>(&decoded) {
                    result_json = Some(val);
                    break;
                }
            } else if let Some(pos) = s.find("#TRN_ERROR=") {
                let payload = &s[pos + "#TRN_ERROR=".len()..];
                err_msg = Some(urlencoding_decode(payload));
                break;
            }
        }
    }

    let _ = win.destroy();

    if let Some(err) = err_msg {
        return Err(err);
    }

    let Some(segments) = result_json.and_then(|v| v.as_array().cloned()) else {
        return Err("Délai de synchronisation dépassé ou profil inaccessible.".into());
    };

    let target_playlist = match focus.as_str() {
        "1v1" => "playlist-10",
        "3v3" => "playlist-13",
        _ => "playlist-11",
    };

    let mut found_segment = None;
    for seg in &segments {
        if seg["attributes"]["playlist"].as_str() == Some(target_playlist) {
            found_segment = Some(seg.clone());
            break;
        }
    }

    if found_segment.is_none() {
        let name_target = match focus.as_str() {
            "1v1" => "Ranked Duel 1v1",
            "3v3" => "Ranked Standard 3v3",
            _ => "Ranked Doubles 2v2",
        };
        for seg in &segments {
            if let Some(name) = seg["metadata"]["name"].as_str() {
                if name.contains(name_target) {
                    found_segment = Some(seg.clone());
                    break;
                }
            }
        }
    }

    let Some(seg) = found_segment else {
        return Err(format!("Aucune statistique classée trouvée pour le mode {focus} sur votre profil."));
    };

    let rating = seg["stats"]["rating"]["value"].as_f64().unwrap_or(0.0);
    let rank_name = seg["stats"]["tier"]["metadata"]["name"].as_str().unwrap_or("Unranked");
    let division_name = seg["stats"]["division"]["metadata"]["name"].as_str().unwrap_or("Division I");

    if rating > 0.0 {
        {
            let ctx = app.state::<Ctx>();
            let mut c = ctx.core.lock().unwrap();
            let is_fresh_session = c.tracker.session.games == 0;
            let old_mmr = c.tracker.session.current_mmr;
            c.tracker.set_mmr(rating, &focus);
            if is_fresh_session {
                c.tracker.session.session_mmr_change = 0.0;
                c.tracker.session.last_match_mmr_change = 0.0;
            } else if old_mmr > 0.0 {
                let diff = rating - old_mmr;
                c.tracker.session.last_match_mmr_change = diff;
                c.tracker.session.session_mmr_change += diff;
            }
            let start_mmr = if is_fresh_session { rating } else { c.store.data.overlay.starting_mmr };
            c.store.update(json!({ "overlay": { "startingMmr": start_mmr } }));
            c.persist_session();
            c.log(&format!("MMR synchronisé en direct depuis Tracker Network : {:.1} ({} {})", rating, rank_name, division_name));
        }
        publish(&app);
    }

    Ok(json!({
        "success": true,
        "mmr": rating,
        "rank": rank_name,
        "division": division_name,
        "playlist": focus
    }))
}

#[tauri::command]
async fn sync_tracker_mmr(app: AppHandle) -> Result<Value, String> {
    perform_tracker_sync(app).await
}

#[tauri::command]
fn get_autostart() -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let mut cmd = std::process::Command::new("reg");
        cmd.args(["query", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run", "/v", "RL Session Stats"]);
        cmd.creation_flags(0x0800_0000);
        let output = cmd.output();
        output.map(|o| o.status.success()).unwrap_or(false)
    }
    #[cfg(not(windows))]
    false
}

#[tauri::command]
fn set_autostart(enabled: bool) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        if enabled {
            if let Ok(exe_path) = std::env::current_exe() {
                let cmd = format!("\"{}\" --minimized", exe_path.to_string_lossy());
                let mut p = std::process::Command::new("reg");
                p.args(["add", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run", "/v", "RL Session Stats", "/t", "REG_SZ", "/d", &cmd, "/f"]);
                p.creation_flags(0x0800_0000);
                let output = p.output();
                return output.map(|o| o.status.success()).unwrap_or(false);
            }
        } else {
            let mut p = std::process::Command::new("reg");
            p.args(["delete", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run", "/v", "RL Session Stats", "/f"]);
            p.creation_flags(0x0800_0000);
            let output = p.output();
            return output.map(|o| o.status.success()).unwrap_or(false);
        }
    }
    false
}

fn handle_exit_cleanup(app: &AppHandle) {
    let ctx = app.state::<Ctx>();
    let mut c = ctx.core.lock().unwrap();
    if c.tracker.session.games > 0 {
        c.archive_session(&c.tracker.session.clone());
    }
    if c.store.data.counting.reset_on_exit {
        c.tracker.reset();
        c.persist_session();
    }
}

#[tauri::command]
fn delete_history_session(app: AppHandle, id: String) -> Value {
    {
        let ctx = app.state::<Ctx>();
        let c = ctx.core.lock().unwrap();
        c.delete_history_item(&id);
    }
    publish(&app);
    get_state(app)
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    handle_exit_cleanup(&app);
    app.exit(0);
}

#[tauri::command]
fn overlay_state(app: AppHandle) -> Value {
    app.state::<Ctx>().core.lock().unwrap().overlay_state()
}

#[tauri::command]
fn overlay_size(app: AppHandle, w: f64, h: f64) {
    if !(w.is_finite() && h.is_finite()) || w <= 0.0 || h <= 0.0 {
        return;
    }
    let changed = {
        let ctx = app.state::<Ctx>();
        let mut c = ctx.core.lock().unwrap();
        let (cur_w, cur_h) = c.overlay_size;
        if (cur_w - w).abs() < 2.0 && (cur_h - h).abs() < 2.0 {
            false
        } else {
            c.overlay_size = (w.ceil(), h.ceil());
            true
        }
    };
    if changed {
        apply_overlay(&app);
    }
}

#[tauri::command]
fn drag_overlay_by(app: AppHandle, dx: f64, dy: f64) {
    let Some(win) = app.get_webview_window("overlay") else { return };
    if let Ok(pos) = win.outer_position() {
        let sf = win.scale_factor().unwrap_or(1.0);
        let new_pos = LogicalPosition::new(pos.x as f64 / sf + dx, pos.y as f64 / sf + dy);
        let _ = win.set_position(new_pos);
    }
}

#[tauri::command]
fn save_overlay_drag_finish(app: AppHandle) {
    let Some(win) = app.get_webview_window("overlay") else { return };
    let sf = win.scale_factor().unwrap_or(1.0);
    if let (Ok(win_pos), Ok(Some(mon))) = (win.outer_position(), win.primary_monitor()) {
        let mon_pos = mon.position().to_logical::<f64>(sf);
        let cur_pos = win_pos.to_logical::<f64>(sf);
        let rel_x = (cur_pos.x - mon_pos.x).round();
        let rel_y = (cur_pos.y - mon_pos.y).round();
        {
            let ctx = app.state::<Ctx>();
            let mut c = ctx.core.lock().unwrap();
            c.store.update(json!({
                "overlay": {
                    "position": "custom",
                    "customX": rel_x,
                    "customY": rel_y
                }
            }));
            c.store.save();
        }
        publish(&app);
    }
}

#[tauri::command]
fn set_overlay_preview(app: AppHandle, preview: bool) -> bool {
    let ctx = app.state::<Ctx>();
    ctx.preview_overlay.store(preview, Ordering::Relaxed);
    apply_overlay(&app);
    preview
}

#[tauri::command]
fn get_overlay_preview(app: AppHandle) -> bool {
    let ctx = app.state::<Ctx>();
    ctx.preview_overlay.load(Ordering::Relaxed)
}

#[tauri::command]
fn app_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

#[tauri::command]
fn set_overlay_custom_position(app: AppHandle, x: f64, y: f64) {
    {
        let ctx = app.state::<Ctx>();
        let mut c = ctx.core.lock().unwrap();
        c.store.update(json!({
            "overlay": {
                "position": "custom",
                "customX": x.round(),
                "customY": y.round()
            }
        }));
        c.store.save();
    }
    apply_overlay(&app);
    publish(&app);
}

// ---------------------------------------------------------------------------
// Démarrage
// ---------------------------------------------------------------------------

fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle().clone();

    let data_dir = app.path().app_data_dir()?;
    fs::create_dir_all(&data_dir)?;
    let log_file = data_dir.join("journal.txt");
    let _ = fs::write(&log_file, "");
    let session_file = data_dir.join("session.json");

    let mut store = SettingsStore::load(data_dir.join("settings.json"));
    let documents_dir = app.path().document_dir().ok();
    if !rl_config::is_valid_install(&store.data.stats_api.install_dir) {
        if let Some(dir) = rl_config::find_install_dirs().into_iter().next() {
            store.update(json!({ "statsApi": { "installDir": dir } }));
        }
    }
    // Détection automatique du pseudo au lancement s'il n'est pas encore renseigné
    if store.data.player.custom_name.trim().is_empty() {
        if let Some(player) = rl_config::detect_player_from_logs(documents_dir.as_deref()) {
            store.update(json!({
                "player": {
                    "platform": player.platform,
                    "customName": player.name
                }
            }));
        }
    }

    let saved_mmr = fs::read_to_string(&session_file)
        .ok()
        .and_then(|t| serde_json::from_str::<Session>(&t).ok())
        .map(|s| s.current_mmr)
        .filter(|&m| m > 0.0)
        .unwrap_or(store.data.overlay.starting_mmr);

    let session = if store.data.counting.reset_on_start {
        let mut s = Session::default();
        s.focused_playlist = store.data.counting.focused_playlist.clone();
        s.current_mmr = saved_mmr;
        let (name, div, tier) = calculate_rank(saved_mmr, &s.focused_playlist);
        s.rank_name = name.to_string();
        s.division = div;
        s.rank_tier = tier;
        s
    } else {
        fs::read_to_string(&session_file)
            .ok()
            .and_then(|t| serde_json::from_str::<Session>(&t).ok())
            .unwrap_or_else(|| {
                let mut s = Session::default();
                s.focused_playlist = store.data.counting.focused_playlist.clone();
                s.current_mmr = saved_mmr;
                let (name, div, tier) = calculate_rank(saved_mmr, &s.focused_playlist);
                s.rank_name = name.to_string();
                s.division = div;
                s.rank_tier = tier;
                s
            })
    };

    let rl_port = Arc::new(AtomicU16::new(store.data.stats_api.port));
    let server_port = store.data.server.port;
    let history_file = data_dir.join("history.json");
    let core = Core {
        store,
        tracker: Tracker::new(session),
        connected: false,
        overlay_size: (320.0, 80.0),
        obs_dir: data_dir.join("obs"),
        session_file,
        history_file,
        log_file,
        documents_dir,
    };
    core.persist_session();
    let core = Arc::new(Mutex::new(core));

    let web = {
        let core = core.clone();
        WebServer::start(server_port, Arc::new(move || core.lock().unwrap().overlay_state()))
    };

    app.manage(Ctx {
        core: core.clone(),
        rl_port: rl_port.clone(),
        web,
        toggle_item: Mutex::new(None),
        preview_overlay: Arc::new(AtomicBool::new(false)),
    });

    // --- Icône près de l'horloge ---
    let enabled = core.lock().unwrap().store.data.overlay.enabled;
    let open = MenuItem::with_id(app, "open", "Ouvrir les paramètres", true, None::<&str>)?;
    let toggle = CheckMenuItem::with_id(app, "toggle", "Afficher l'overlay en jeu (Ctrl+Maj+O)", true, enabled, None::<&str>)?;
    let reset = MenuItem::with_id(app, "reset", "Réinitialiser la session", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quitter", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open, &toggle, &reset, &separator, &quit])?;
    *app.state::<Ctx>().toggle_item.lock().unwrap() = Some(toggle);

    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("RL Session Stats")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => open_settings(app),
            "toggle" => toggle_overlay(app),
            "reset" => reset_session(app),
            "quit" => {
                handle_exit_cleanup(app);
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                open_settings(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;

    // --- Fenêtre overlay : transparente, toujours au premier plan, les clics la traversent ---
    let overlay = WebviewWindowBuilder::new(app, "overlay", WebviewUrl::App("overlay.html".into()))
        .title("RL Session Stats - Overlay")
        .inner_size(320.0, 80.0)
        .position(OFFSCREEN, OFFSCREEN)
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(true)
        .focused(false)
        .visible(true)
        .build()?;
    overlay.set_ignore_cursor_events(true)?;

    // --- Raccourci global Ctrl+Maj+O ---
    {
        use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
        let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyO);
        app.handle().plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, pressed, event| {
                    if pressed == &shortcut && event.state() == ShortcutState::Pressed {
                        toggle_overlay(app);
                    }
                })
                .build(),
        )?;
        if app.global_shortcut().register(shortcut).is_err() {
            core.lock().unwrap().log("Raccourci Ctrl+Maj+O indisponible (déjà utilisé par une autre appli)");
        }
    }

    // --- Connexion à la Stats API ---
    {
        let app = handle.clone();
        stats_client::spawn(rl_port, move |event| {
            let effects = {
                let ctx = app.state::<Ctx>();
                let mut c = ctx.core.lock().unwrap();
                match event {
                    ClientEvent::Connected => {
                        c.connected = true;
                        c.log("Connecté à Rocket League");
                        Effects { status_changed: true, ..Default::default() }
                    }
                    ClientEvent::Disconnected => {
                        c.connected = false;
                        let rules = c.rules();
                        let mut e = c.tracker.connection_lost(&rules);
                        e.status_changed = true;
                        e
                    }
                    ClientEvent::Message(name, data) => {
                        let rules = c.rules();
                        let identity = c.identity();
                        c.tracker.handle(&name, &data, identity.as_deref(), &rules)
                    }
                }
            };
            after_effects(&app, effects);
        });
    }

    // Game Watcher : veille automatique quand le jeu est fermé ou en arrière-plan (Alt-Tab)
    {
        let app = handle.clone();
        thread::spawn(move || {
            let mut was_running = false;
            let mut was_visible = false;
            let mut run_check_counter: u32 = 0;
            let mut is_running = false;

            loop {
                // Vérifier si le processus Rocket League tourne toutes les ~1.5 secondes
                if run_check_counter % 6 == 0 {
                    let running_now = rl_config::is_rocket_league_running();
                    if running_now && !was_running {
                        was_running = true;
                        {
                            let ctx = app.state::<Ctx>();
                            ctx.core.lock().unwrap().log("Rocket League détecté");
                        }
                        let h = app.clone();
                        tauri::async_runtime::spawn(async move {
                            // Attente de 2.5s pour laisser Rocket League démarrer et écrire Launch.log
                            std::thread::sleep(Duration::from_millis(2500));
                            let docs = h.state::<Ctx>().core.lock().unwrap().documents_dir.clone();
                            if let Some(player) = rl_config::detect_player_from_logs(docs.as_deref()) {
                                {
                                    let ctx = h.state::<Ctx>();
                                    let mut c = ctx.core.lock().unwrap();
                                    c.store.update(json!({
                                        "player": {
                                            "platform": player.platform,
                                            "customName": player.name
                                        }
                                    }));
                                    c.log(&format!("Profil joueur actualisé : {} ({})", player.name, player.platform));
                                }
                                publish(&h);
                            }
                            let auto_sync = h.state::<Ctx>().core.lock().unwrap().store.data.player.auto_sync_mmr;
                            if auto_sync {
                                std::thread::sleep(Duration::from_millis(1500));
                                let _ = perform_tracker_sync(h).await;
                            }
                        });
                    } else if !running_now && was_running {
                        was_running = false;
                        {
                            let ctx = app.state::<Ctx>();
                            ctx.core.lock().unwrap().log("Rocket League fermé — mise en veille de l'overlay");
                        }
                    }
                    is_running = running_now;
                }
                run_check_counter = run_check_counter.wrapping_add(1);

                let (enabled, only_in_game, is_preview) = {
                    let ctx = app.state::<Ctx>();
                    let c = ctx.core.lock().unwrap();
                    (c.store.data.overlay.enabled, c.store.data.overlay.only_in_game, ctx.preview_overlay.load(Ordering::Relaxed))
                };

                let should_be_visible = if is_preview {
                    true
                } else if !enabled {
                    false
                } else if !only_in_game {
                    is_running
                } else {
                    rl_config::is_rocket_league_focused()
                };

                if should_be_visible {
                    if !was_visible {
                        was_visible = true;
                        render_overlay_on_screen(&app);
                    } else if let Some(win) = app.get_webview_window("overlay") {
                        let _ = win.set_always_on_top(true);
                    }
                } else {
                    if was_visible {
                        was_visible = false;
                        hide_overlay(&app);
                    }
                }

                thread::sleep(Duration::from_millis(250));
            }
        });
    }

    // Synchronisation automatique du MMR au lancement si un joueur est déjà configuré
    {
        let app = handle.clone();
        tauri::async_runtime::spawn(async move {
            thread::sleep(Duration::from_secs(2));
            let has_user = {
                let ctx = app.state::<Ctx>();
                let c = ctx.core.lock().unwrap();
                let p = &c.store.data.player;
                !p.custom_name.trim().is_empty() || !p.primary_id.trim().is_empty() || !p.detected_name.trim().is_empty()
            };
            if has_user {
                let _ = perform_tracker_sync(app).await;
            }
        });
    }

    if !std::env::args().any(|a| a == "--minimized") {
        if let Some(settings_win) = app.get_webview_window("settings") {
            let _ = settings_win.show();
            let _ = settings_win.set_focus();
        }
    }

    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| open_settings(app)))
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_state,
            save_settings,
            reset_session_cmd,
            set_mmr_cmd,
            statsapi_status,
            statsapi_detect,
            statsapi_select_platform,
            detect_player_now,
            statsapi_choose,
            statsapi_enable,
            open_obs_folder,
            open_browser_url,
            sync_tracker_mmr,
            delete_history_session,
            get_autostart,
            set_autostart,
            quit_app,
            overlay_state,
            overlay_size,
            drag_overlay_by,
            save_overlay_drag_finish,
            get_overlay_preview,
            set_overlay_preview,
            app_version,
            set_overlay_custom_position,
        ])
        .on_window_event(|window, event| {
            // Fermer la fenêtre des paramètres la cache seulement : l'overlay reste actif.
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "settings" {
                    api.prevent_close();
                    let _ = window.hide();
                    let ctx = window.app_handle().state::<Ctx>();
                    if ctx.preview_overlay.swap(false, Ordering::Relaxed) {
                        apply_overlay(window.app_handle());
                    }
                }
            }
        })
        .setup(setup)
        .run(tauri::generate_context!())
        .expect("Impossible de démarrer RL Session Stats");
}
