# RL Session Stats

<p align="center">
  <img src="ui/icon.png" width="96" height="96" alt="RL Session Stats Logo">
</p>

<p align="center">
  <strong>A modern, customizable replacement for RocketStats in Rocket League, built for the Easy Anti-Cheat era.</strong>
</p>

<p align="center">
  <a href="https://github.com/NeyorDEV/rl-session-stats/releases/latest">
    <img src="https://img.shields.io/github/v/release/NeyorDEV/rl-session-stats?logo=github&color=blue" alt="Latest Release">
  </a>
  <a href="https://github.com/NeyorDEV/rl-session-stats/actions/workflows/build.yml">
    <img src="https://github.com/NeyorDEV/rl-session-stats/actions/workflows/build.yml/badge.svg" alt="Build Status">
  </a>
  <a href="LICENSE">
    <img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License: MIT">
  </a>
  <a href="https://tauri.app/">
    <img src="https://img.shields.io/badge/Tauri-v2-24C8D8.svg?logo=tauri&logoColor=white" alt="Tauri 2">
  </a>
  <img src="https://img.shields.io/badge/Easy_Anti--Cheat-100%25_Safe_(No_Injection)-brightgreen.svg" alt="EAC Safe">
  <img src="https://img.shields.io/badge/Platform-Windows-0078D6.svg?logo=windows&logoColor=white" alt="Platform: Windows">
</p>

---

## 🛡️ 100% Safe with Easy Anti-Cheat (No Ban Risk)

Ever since **Easy Anti-Cheat (EAC)** was introduced in Rocket League, traditional BakkesMod plugins and memory injectors are blocked or risk permanent account bans.

**RL Session Stats uses ZERO injection, ZERO DLL hooking, and ZERO memory reading.**  
It exclusively communicates through Rocket League's **official native local telemetry Stats API** (`TAGame.MatchStatsExporter_TA`) via a local TCP socket on `127.0.0.1:49123`. You can play online competitive matches normally with EAC fully enabled without any risk.

---

## ✨ Features

* **🎨 RocketStats Theme & Multiple Layouts**:
  * **RocketStats (Classic)**: Faithful homage to the beloved RocketStats BakkesMod plugin, featuring high-resolution SVG rank badges, large MMR readout, match delta indicator, and win/loss/streak counters.
  * **Modern Ribbon**: Sleek angled cells with individual metrics.
  * **Vertical Card**: Compact side-widget format.
  * **Minimalist**: Pure text & icon layout with shadow effects.
* **📈 MMR & Rank Tracking**:
  * Calculates rank tier (Bronze I through Supersonic Legend) and division in real-time.
  * Live session MMR and match delta updates (+9.0 / -9.0 average per ranked match).
  * Custom starting MMR and custom delta values configurable from the settings.
* **👤 Player Platform Selector (Epic Games / Steam)**:
  * Select your platform (Epic or Steam) to unlock your in-game username input.
  * Direct pseudo matching without having to look up complicated account IDs.
  * Also supports fully automatic camera-based detection.
* **🎛️ Deep Customization**:
  * Adjust **Opacity** (20% to 100%) and **Scale** (60% to 200%).
  * Screen positioning (Top-Left, Top-Right, Bottom-Left, etc.) with custom pixel offsets.
  * Toggle individual stats on/off: Rank, MMR, Match MMR, Session MMR, Gamemode, Wins, Losses, Streak, Winrate, Goals, Saves, Demos, etc.
* **🎥 OBS & Streamlabs Ready**:
  * **Browser Source**: Stream real-time stats via the local web server (`http://localhost:5177/overlay`) powered by Server-Sent Events (SSE).
  * **Text Files**: Automatically generates and updates `.txt` files for text sources (`mmr.txt`, `mmr_change.txt`, `mmr_session.txt`, `rank.txt`, `division.txt`, `rank_full.txt`, `wins.txt`, `losses.txt`, `streak.txt`, etc.).
* **⚡ Ultra Lightweight**: Built on **Tauri 2 (Rust)** and vanilla web technologies with minimal CPU and RAM footprint.

---

## 🚀 Download & Quick Start (Zero-Install)

You can download pre-compiled, ready-to-use Windows binaries directly from the **[Latest Release](https://github.com/NeyorDEV/rl-session-stats/releases/latest)**:

* **[RL Session Stats (portable).exe](https://github.com/NeyorDEV/rl-session-stats/releases/latest/download/RL.Session.Stats.portable.exe)** : Standalone portable executable (no installation required, simply double-click and run).
* **[RL Session Stats Setup (.exe)](https://github.com/NeyorDEV/rl-session-stats/releases/latest/download/RL.Session.Stats_1.0.0_x64-setup.exe)** : Standard Windows installer with desktop and Start menu shortcuts.

> *Note: Windows SmartScreen may show an untrusted warning on the first run for unsigned open-source binaries. Click **More info** → **Run anyway**.*

---

## 🎮 First-Time Setup

1. **Close Rocket League** before enabling the API.
2. Launch **RL Session Stats**.
3. Under the **Rocket League Stats API** section:
   - Click **Activer la Stats API** (*Enable Stats API*). The application will automatically locate your Steam or Epic Games installation and configure `DefaultStatsAPI.ini` (a `.bak` backup is preserved).
   - If your installation isn't detected automatically, click **Choisir le dossier…** and select your `rocketleague` root directory.
4. Open Rocket League and go to:
   - **Settings → Video → Window Mode: Borderless** (required for transparent overlays to render on top of the game).
5. Play a game! Your player identity is recognized automatically on your first match.

---

## ⌨️ Controls & Shortcuts

* <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>O</kbd> : Toggle overlay visibility in-game.
* **Tray Icon**: Right-click the system tray icon near your clock to open settings, reset session, or quit.
* **Reset Session**: Two-step confirmation button to reset session stats.

---

## 🎥 OBS Studio / Streamlabs Setup

* **Browser Source**:
  - Add a new **Browser** source.
  - URL: `http://localhost:5177/overlay`
  - Width: `750` | Height: `180`
* **Text Files**:
  - Click **Ouvrir le dossier** in the settings to access your OBS text files folder:
    - `mmr.txt`
    - `mmr_change.txt`
    - `rank_full.txt` (`Diamond I Div. 2`)
    - `wins.txt`, `losses.txt`, `streak.txt`, `winrate.txt`, etc.

---

## 🛠️ Building from Source

If you want to compile locally:

### Prerequisites
* [Node.js](https://nodejs.org/) (v18+)
* [Rust](https://rustup.rs/) (stable toolchain)
* Windows C++ Build Tools (via Visual Studio Installer)

### Development
```powershell
# Clone the repository
git clone https://github.com/NeyorDEV/rl-session-stats.git
cd rl-session-stats

# Install Tauri CLI
npm install

# Run automated tests
cargo test --manifest-path src-tauri/Cargo.toml

# Start development mode
npx tauri dev

# Build production bundle (.exe & setup)
npx tauri build
```

---

## 📄 License

This project is licensed under the [MIT License](LICENSE) - see the LICENSE file for details.
