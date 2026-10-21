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

## 100% Safe with Easy Anti-Cheat (No Ban Risk)

Ever since **Easy Anti-Cheat (EAC)** was introduced in Rocket League, traditional BakkesMod plugins and memory injectors are blocked or risk permanent account bans.

**RL Session Stats uses ZERO injection, ZERO DLL hooking, and ZERO memory reading.**  
It exclusively communicates through Rocket League's **official native local telemetry Stats API** (`TAGame.MatchStatsExporter_TA`) via a local TCP socket on `127.0.0.1:49123`. You can play online competitive matches normally with EAC fully enabled without any risk.

---

## Features

* **RocketStats Theme & Multiple Layouts**:
  * **RocketStats (Classic)**: Faithful homage to the beloved RocketStats BakkesMod plugin, featuring high-resolution SVG rank badges, large MMR readout, match delta indicator, and win/loss/streak counters.
  * **Modern Ribbon**: Sleek angled cells with individual metrics.
  * **Vertical Card**: Compact side-widget format.
  * **Minimalist**: Pure text & icon layout with shadow effects.
* **MMR & Rank Tracking**:
  * **Accurate Official Brackets**: Exact MMR curves for 1v1, 2v2, and 3v3 (e.g. 1656 ELO = Grand Champion II Div. 3).
  * **Focused Playlist Selector**: Focus on 1v1 (Duel), 2v2 (Doubles), or 3v3 (Standard) with instant tier & division recalculation.
  * **HD Modern Rank Icons**: Authentic transparent 256x256 post-F2P badges (including ruby GC1, GC2, GC3, and Supersonic Legend).
  * Live session MMR and match delta updates (+9.0 / -9.0 average per ranked match).
  * Custom starting MMR and custom delta values configurable from the settings.
  * Direct 1-click link to view your live profile on Rocket League Tracker.
* **Player Platform Selector (Epic Games / Steam)**:
  * Select your platform (Epic or Steam) to unlock your in-game username input.
  * Direct pseudo matching without having to look up complicated account IDs.
  * Also supports fully automatic camera-based detection.
* **Deep Customization**:
  * Adjust **Opacity** (20% to 100%) and **Scale** (60% to 200%).
  * Screen positioning (Top-Left, Top-Right, Bottom-Left, etc.) with custom pixel offsets.
  * Toggle individual stats on/off: Rank, MMR, Match MMR, Session MMR, Gamemode, Wins, Losses, Streak, Winrate, Goals, Saves, Demos, etc.
* **OBS & Streamlabs Ready**:
  * **Browser Source**: Stream real-time stats via the local web server (`http://localhost:5177/overlay`) powered by Server-Sent Events (SSE).
  * **Text Files**: Automatically generates and updates `.txt` files for text sources (`mmr.txt`, `mmr_change.txt`, `mmr_session.txt`, `rank.txt`, `division.txt`, `rank_full.txt`, `wins.txt`, `losses.txt`, `streak.txt`, etc.).
* **Ultra Lightweight**: Built on **Tauri 2 (Rust)** and vanilla web technologies with minimal CPU and RAM footprint.

---

## Download & Quick Start (Zero-Install)

You can download pre-compiled, ready-to-use Windows binaries directly from the **[Latest Release](https://github.com/NeyorDEV/rl-session-stats/releases/latest)**:

* **[RL Session Stats (portable).exe](https://github.com/NeyorDEV/rl-session-stats/releases/latest/download/RL.Session.Stats.portable.exe)** : Standalone portable executable (no installation required, simply double-click and run).
* **[RL Session Stats Setup (.exe)](https://github.com/NeyorDEV/rl-session-stats/releases/latest/download/RL.Session.Stats_1.2.0_x64-setup.exe)** : Standard Windows installer with desktop and Start menu shortcuts.

> *Note: Windows SmartScreen may show an untrusted warning on the first run for unsigned open-source binaries. Click **More info** -> **Run anyway**.*

---

## Quick Start & Setup Walkthrough

Get everything up and running in 2 minutes:

1. **Close Rocket League** before enabling the API *(important: Rocket League overwrites configuration files upon exiting)*.
2. **Launch `RL Session Stats`**:
   - In the **Rocket League Stats API** section, click **Activer la Stats API** (*Enable Stats API*). The application automatically locates your Steam or Epic Games installation and configures `DefaultStatsAPI.ini` (a `.bak` backup is preserved).
3. **Configure your account & playlist**:
   - Under the **Player & Account** (*Joueur & Compte*) section:
     - Select **Epic Games** or **Steam**.
     - Type your exact **in-game username**.
     - Select your focused playlist: **1v1 (Duel)**, **2v2 (Doubles)**, or **3v3 (Standard)**.
     - Enter your **starting MMR** (e.g. `1656` for **Grand Champion II Division 3**) and click **Appliquer** (*Apply*). *(Optional: click "Ouvrir mon profil Rocket League Tracker" to inspect your live web stats directly in your browser).*
4. **Launch Rocket League** and configure video settings:
   - Navigate to **Options -> Video -> Window Mode: Borderless**. This allows Windows to render transparent overlays on top of the game.
5. **Play!**
   - Your player identity is recognized automatically on your first match.
   - At the end of each game, your wins, losses, goals, saves, and MMR (**+9 / -9**) update and display in real time.
   - Your MMR and rank are continuously saved across sessions: you never need to retype them!

> **Pro Tip (Auto-start with Windows)**: To never worry about launching the tool before playing, press <kbd>Win</kbd> + <kbd>R</kbd>, type `shell:startup`, and paste a shortcut to `RL Session Stats (portable).exe`. The app will launch silently in your system tray on boot (~0% CPU) and will always be synced!

---

## Controls & Shortcuts

* <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>O</kbd> : Toggle overlay visibility in-game.
* **Tray Icon**: Right-click the system tray icon near your clock to open settings, reset session, or quit.
* **Reset Session**: Two-step confirmation button to reset session stats.

---

## OBS Studio / Streamlabs Setup

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

## Building from Source

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
npx @tauri-apps/cli dev

# Build production bundle (.exe & setup)
npx @tauri-apps/cli build
```

---

## License

This project is licensed under the [MIT License](LICENSE) - see the LICENSE file for details.
