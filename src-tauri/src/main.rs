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
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent, Wry,
};
use tauri_plugin_dialog::DialogExt;

use session::{CountRules, Effects, Session, Tracker, PLAYLISTS, RANKED};
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
    log_file: PathBuf,
    documents_dir: Option<PathBuf>,
}

impl Core {
    fn rules(&self) -> CountRules {
        let c = &self.store.data.counting;
        CountRules {
            online_only: c.online_only,
            mode: c.mode.clone(),
            playlists: c.playlists.clone(),
            count_leave_as_loss: c.count_leave_as_loss,
        }
    }

    fn identity(&self) -> Option<String> {
        let p = &self.store.data.player;
        let id = if p.primary_id.is_empty() { &p.detected_id } else { &p.primary_id };
        (!id.is_empty()).then(|| id.clone())
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

    fn full_state(&self, server_error: Option<String>) -> Value {
        let playlists: serde_json::Map<String, Value> =
            PLAYLISTS.iter().map(|(id, name)| (id.to_string(), json!(name))).collect();
        json!({
            "settings": self.store.data,
            "session": self.tracker.session,
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
        format!("RL Session Stats - {}V / {}D - streak {}", s.wins, s.losses, fmt_streak(s.streak))
    }

    /// Sauvegarde la session et écrit les fichiers texte pour OBS.
    fn persist_session(&self) {
        let s = &self.tracker.session;
        if let Ok(text) = serde_json::to_string_pretty(s) {
            let _ = fs::write(&self.session_file, text);
        }
        let _ = fs::create_dir_all(&self.obs_dir);
        let winrate = if s.games > 0 { format!("{}%", (100 * s.wins + s.games / 2) / s.games) } else { "-".into() };
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
        }
    }
    if effects.session_changed || effects.status_changed || effects.detected.is_some() {
        publish(app);
    }
}

// ---------------------------------------------------------------------------
// Overlay en jeu
// ---------------------------------------------------------------------------

fn apply_overlay(app: &AppHandle) {
    let Some(win) = app.get_webview_window("overlay") else { return };
    let (o, (w, h)) = {
        let ctx = app.state::<Ctx>();
        let c = ctx.core.lock().unwrap();
        (c.store.data.overlay.clone(), c.overlay_size)
    };
    if !o.enabled {
        let _ = win.set_position(LogicalPosition::new(OFFSCREEN, OFFSCREEN));
        return;
    }
    let Some(monitor) = win.primary_monitor().ok().flatten() else { return };
    let sf = monitor.scale_factor();
    let pos = monitor.position().to_logical::<f64>(sf);
    let size = monitor.size().to_logical::<f64>(sf);
    let (w, h) = (w.max(40.0), h.max(20.0));
    let (vertical, horizontal) = o.position.split_once('-').unwrap_or(("top", "left"));

    let x = match horizontal {
        "left" => pos.x + o.offset_x,
        "right" => pos.x + size.width - w - o.offset_x,
        _ => pos.x + (size.width - w) / 2.0 + o.offset_x,
    };
    let y = if vertical == "bottom" { pos.y + size.height - h - o.offset_y } else { pos.y + o.offset_y };

    let _ = win.set_size(LogicalSize::new(w, h));
    let _ = win.set_position(LogicalPosition::new(x.round(), y.round()));
    let _ = win.set_always_on_top(true);
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
        ctx.rl_port.store(c.store.data.stats_api.port, Ordering::SeqCst);
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
    let found = rl_config::find_install_dirs();
    if let Some(dir) = found.first() {
        let ctx = app.state::<Ctx>();
        ctx.core.lock().unwrap().store.update(json!({ "statsApi": { "installDir": dir } }));
    }
    publish(&app);
    let mut status = stats_api_status(&app);
    status["notFound"] = json!(found.is_empty());
    status
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
fn quit_app(app: AppHandle) {
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
    app.state::<Ctx>().core.lock().unwrap().overlay_size = (w.ceil(), h.ceil());
    apply_overlay(&app);
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
    if !rl_config::is_valid_install(&store.data.stats_api.install_dir) {
        if let Some(dir) = rl_config::find_install_dirs().into_iter().next() {
            store.update(json!({ "statsApi": { "installDir": dir } }));
        }
    }

    let session = if store.data.counting.reset_on_start {
        Session::default()
    } else {
        fs::read_to_string(&session_file)
            .ok()
            .and_then(|t| serde_json::from_str::<Session>(&t).ok())
            .unwrap_or_default()
    };

    let rl_port = Arc::new(AtomicU16::new(store.data.stats_api.port));
    let server_port = store.data.server.port;
    let core = Core {
        store,
        tracker: Tracker::new(session),
        connected: false,
        overlay_size: (320.0, 80.0),
        obs_dir: data_dir.join("obs"),
        session_file,
        log_file,
        documents_dir: app.path().document_dir().ok(),
    };
    core.persist_session();
    let core = Arc::new(Mutex::new(core));

    let web = {
        let core = core.clone();
        WebServer::start(server_port, Arc::new(move || core.lock().unwrap().overlay_state()))
    };

    app.manage(Ctx { core: core.clone(), rl_port: rl_port.clone(), web, toggle_item: Mutex::new(None) });

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
            "quit" => app.exit(0),
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
        .resizable(false)
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

    // Réaffirme régulièrement le « toujours au premier plan » (certains jeux passent devant).
    {
        let app = handle.clone();
        thread::spawn(move || loop {
            thread::sleep(Duration::from_secs(3));
            let enabled = app.state::<Ctx>().core.lock().unwrap().store.data.overlay.enabled;
            if enabled {
                if let Some(win) = app.get_webview_window("overlay") {
                    let _ = win.set_always_on_top(true);
                }
            }
        });
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
            statsapi_status,
            statsapi_detect,
            statsapi_choose,
            statsapi_enable,
            open_obs_folder,
            quit_app,
            overlay_state,
            overlay_size,
        ])
        .on_window_event(|window, event| {
            // Fermer la fenêtre des paramètres la cache seulement : l'overlay reste actif.
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "settings" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .setup(setup)
        .run(tauri::generate_context!())
        .expect("Impossible de démarrer RL Session Stats");
}
