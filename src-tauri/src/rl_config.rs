//! Trouve le dossier d'installation de Rocket League et règle la Stats API dans ses fichiers .ini.

use serde_json::{json, Value};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

const SECTION: &str = "[TAGame.MatchStatsExporter_TA]";

#[cfg(windows)]
fn hide_console(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
}
#[cfg(not(windows))]
fn hide_console(_cmd: &mut Command) {}

pub fn is_valid_install(dir: &str) -> bool {
    !dir.is_empty() && Path::new(dir).join("TAGame").join("Config").is_dir()
}

/// Accepte aussi un sous-dossier (TAGame, Config, Binaries...) choisi par erreur.
pub fn normalize_install_dir(dir: &Path) -> Option<PathBuf> {
    let mut current = Some(dir);
    for _ in 0..4 {
        let c = current?;
        if c.join("TAGame").join("Config").is_dir() {
            return Some(c.to_path_buf());
        }
        current = c.parent();
    }
    None
}

fn epic_install_dirs() -> Vec<String> {
    let manifests = Path::new(r"C:\ProgramData\Epic\EpicGamesLauncher\Data\Manifests");
    let Ok(entries) = fs::read_dir(manifests) else { return vec![] };
    entries
        .flatten()
        .filter(|e| e.path().extension().map_or(false, |x| x == "item"))
        .filter_map(|e| fs::read_to_string(e.path()).ok())
        .filter_map(|text| serde_json::from_str::<Value>(&text).ok())
        .filter(|item| {
            let name = format!(
                "{} {}",
                item["DisplayName"].as_str().unwrap_or(""),
                item["AppName"].as_str().unwrap_or("")
            )
            .to_lowercase();
            name.contains("rocket league") || item["AppName"] == "Sugar"
        })
        .filter_map(|item| item["InstallLocation"].as_str().map(String::from))
        .collect()
}

fn steam_roots() -> Vec<String> {
    let mut roots = vec![r"C:\Program Files (x86)\Steam".to_string(), r"C:\Program Files\Steam".to_string()];
    let mut cmd = Command::new("reg");
    cmd.args(["query", r"HKCU\Software\Valve\Steam", "/v", "SteamPath"]);
    hide_console(&mut cmd);
    if let Ok(out) = cmd.output() {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            if let Some((_, value)) = line.split_once("REG_SZ") {
                roots.insert(0, value.trim().replace('/', "\\"));
            }
        }
    }
    roots
}

fn steam_install_dirs() -> Vec<String> {
    let mut dirs = vec![];
    for root in steam_roots() {
        let mut libraries = vec![root.clone()];
        if let Ok(vdf) = fs::read_to_string(Path::new(&root).join("steamapps").join("libraryfolders.vdf")) {
            for line in vdf.lines() {
                let parts: Vec<&str> = line.split('"').collect();
                if parts.len() >= 4 && parts[1] == "path" {
                    libraries.push(parts[3].replace(r"\\", r"\"));
                }
            }
        }
        for lib in libraries {
            dirs.push(Path::new(&lib).join("steamapps").join("common").join("rocketleague").to_string_lossy().into_owned());
        }
    }
    dirs
}

pub fn find_install_dirs() -> Vec<String> {
    let mut candidates = epic_install_dirs();
    candidates.extend(steam_install_dirs());
    candidates.push(r"C:\Program Files\Epic Games\rocketleague".into());
    candidates.push(r"D:\Epic Games\rocketleague".into());

    let mut seen = std::collections::HashSet::new();
    candidates
        .into_iter()
        .filter(|d| seen.insert(d.to_lowercase()) && is_valid_install(d))
        .collect()
}

/// Fichiers à modifier : ceux qui existent parmi TAStatsAPI.ini / DefaultStatsAPI.ini (dossier du jeu)
/// et TAStatsAPI.ini (Documents). DefaultStatsAPI.ini est créé si aucun n'existe.
fn target_files(install_dir: &str, documents_dir: Option<&Path>) -> Vec<PathBuf> {
    let config = Path::new(install_dir).join("TAGame").join("Config");
    let ta = config.join("TAStatsAPI.ini");
    let def = config.join("DefaultStatsAPI.ini");
    let mut files: Vec<PathBuf> = [ta, def.clone()].into_iter().filter(|f| f.exists()).collect();
    if files.is_empty() {
        files.push(def);
    }
    if let Some(docs) = documents_dir {
        let doc = docs.join("My Games").join("Rocket League").join("TAGame").join("Config").join("TAStatsAPI.ini");
        if doc.exists() {
            files.push(doc);
        }
    }
    files
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Encoding {
    Utf8,
    Utf8Bom,
    Utf16Bom,
}

fn read_ini(path: &Path) -> (String, Encoding) {
    let Ok(bytes) = fs::read(path) else { return (String::new(), Encoding::Utf8) };
    if bytes.starts_with(&[0xFF, 0xFE]) {
        let units: Vec<u16> = bytes[2..].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        (String::from_utf16_lossy(&units), Encoding::Utf16Bom)
    } else if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        (String::from_utf8_lossy(&bytes[3..]).into_owned(), Encoding::Utf8Bom)
    } else {
        (String::from_utf8_lossy(&bytes).into_owned(), Encoding::Utf8)
    }
}

fn write_ini(path: &Path, text: &str, encoding: Encoding) -> io::Result<()> {
    let bytes = match encoding {
        Encoding::Utf8 => text.as_bytes().to_vec(),
        Encoding::Utf8Bom => [&[0xEF, 0xBB, 0xBF][..], text.as_bytes()].concat(),
        Encoding::Utf16Bom => {
            let mut out = vec![0xFF, 0xFE];
            for unit in text.encode_utf16() {
                out.extend_from_slice(&unit.to_le_bytes());
            }
            out
        }
    };
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, bytes)
}

fn is_section_header(line: &str) -> bool {
    let t = line.trim();
    t.starts_with('[') && t.ends_with(']')
}

fn section_range(lines: &[String]) -> Option<(usize, usize)> {
    let start = lines.iter().position(|l| l.trim().eq_ignore_ascii_case(SECTION))?;
    let end = lines
        .iter()
        .enumerate()
        .skip(start + 1)
        .find(|(_, l)| is_section_header(l))
        .map_or(lines.len(), |(i, _)| i);
    Some((start, end))
}

/// Si `line` est de la forme `key = valeur` (insensible à la casse), renvoie la valeur.
fn key_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let (k, v) = line.split_once('=')?;
    k.trim().eq_ignore_ascii_case(key).then(|| v.trim())
}

pub fn get_ini_value(text: &str, key: &str) -> Option<String> {
    let lines: Vec<String> = text.lines().map(String::from).collect();
    let (start, end) = section_range(&lines)?;
    lines[start + 1..end].iter().find_map(|l| key_value(l, key).map(String::from))
}

pub fn set_ini_values(text: &str, values: &[(&str, String)]) -> String {
    let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut lines: Vec<String> = text.lines().map(String::from).collect();
    let (start, mut end) = match section_range(&lines) {
        Some(range) => range,
        None => {
            while lines.last().map_or(false, |l| l.trim().is_empty()) {
                lines.pop();
            }
            if !lines.is_empty() {
                lines.push(String::new());
            }
            lines.push(SECTION.to_string());
            (lines.len() - 1, lines.len())
        }
    };
    for (key, value) in values {
        let line = format!("{key}={value}");
        if let Some(i) = (start + 1..end).find(|&i| key_value(&lines[i], key).is_some()) {
            lines[i] = line;
        } else {
            let mut insert_at = end;
            while insert_at > start + 1 && lines[insert_at - 1].trim().is_empty() {
                insert_at -= 1;
            }
            lines.insert(insert_at, line);
            end += 1;
        }
    }
    let mut out = lines.join(eol);
    out.push_str(eol);
    out
}

pub fn read_status(install_dir: &str, documents_dir: Option<&Path>) -> Value {
    if !is_valid_install(install_dir) {
        return json!({ "installDir": install_dir, "valid": false, "enabled": false, "files": [] });
    }
    let files: Vec<Value> = target_files(install_dir, documents_dir)
        .iter()
        .map(|path| {
            let (text, _) = read_ini(path);
            let rate = get_ini_value(&text, "PacketSendRate").and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
            let port = get_ini_value(&text, "Port").and_then(|v| v.parse::<u16>().ok()).unwrap_or(49123);
            json!({ "path": path, "exists": path.exists(), "packetSendRate": rate, "port": port })
        })
        .collect();
    let existing: Vec<&Value> = files.iter().filter(|f| f["exists"] == true).collect();
    let enabled = !existing.is_empty() && existing.iter().all(|f| f["packetSendRate"].as_f64().unwrap_or(0.0) > 0.0);
    json!({ "installDir": install_dir, "valid": true, "enabled": enabled, "files": files })
}

pub fn enable_stats_api(install_dir: &str, documents_dir: Option<&Path>, packet_send_rate: u32, port: u16) -> io::Result<Vec<PathBuf>> {
    let mut written = vec![];
    for path in target_files(install_dir, documents_dir) {
        let backup = PathBuf::from(format!("{}.bak", path.display()));
        if path.exists() && !backup.exists() {
            fs::copy(&path, &backup)?;
        }
        let (text, encoding) = read_ini(&path);
        let updated = set_ini_values(&text, &[("PacketSendRate", packet_send_rate.to_string()), ("Port", port.to_string())]);
        write_ini(&path, &updated, encoding)?;
        written.push(path);
    }
    Ok(written)
}

#[cfg(windows)]
pub fn is_rocket_league_running() -> bool {
    #[repr(C)]
    struct PROCESSENTRY32W {
        dw_size: u32,
        cnt_usage: u32,
        th32_process_id: u32,
        th32_default_heap_id: usize,
        th32_module_id: u32,
        cnt_threads: u32,
        th32_parent_process_id: u32,
        pc_pri_class_base: i32,
        dw_flags: u32,
        sz_exe_file: [u16; 260],
    }

    extern "system" {
        fn CreateToolhelp32Snapshot(dw_flags: u32, th32_process_id: u32) -> *mut std::ffi::c_void;
        fn Process32FirstW(h_snapshot: *mut std::ffi::c_void, lppe: *mut PROCESSENTRY32W) -> i32;
        fn Process32NextW(h_snapshot: *mut std::ffi::c_void, lppe: *mut PROCESSENTRY32W) -> i32;
        fn CloseHandle(h_object: *mut std::ffi::c_void) -> i32;
    }

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(0x00000002 /* TH32CS_SNAPPROCESS */, 0);
        if snapshot.is_null() || snapshot as isize == -1 {
            return false;
        }

        let mut entry = std::mem::zeroed::<PROCESSENTRY32W>();
        entry.dw_size = std::mem::size_of::<PROCESSENTRY32W>() as u32;

        let mut found = false;
        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                let len = entry.sz_exe_file.iter().position(|&c| c == 0).unwrap_or(entry.sz_exe_file.len());
                let exe = String::from_utf16_lossy(&entry.sz_exe_file[..len]).to_lowercase();
                if exe == "rocketleague.exe" {
                    found = true;
                    break;
                }
                if Process32NextW(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snapshot);
        found
    }
}

#[cfg(not(windows))]
pub fn is_rocket_league_running() -> bool {
    false
}
