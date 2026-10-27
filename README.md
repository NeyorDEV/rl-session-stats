# RL Session Stats

> **Merged project: this repository is no longer updated.**
> RL Session Stats and [LobbyTracker](https://github.com/NeyorDEV/lobbytracker) have been merged into **[RL Tracker Suite](https://github.com/NeyorDEV/rl-tracker-suite)**, a single app with a *Session Stats* tab (everything in this project) and a *LobbyTracker* tab.
> This repository will not receive any further updates: development continues only in [RL Tracker Suite](https://github.com/NeyorDEV/rl-tracker-suite).
>
> - New project: [NeyorDEV/rl-tracker-suite](https://github.com/NeyorDEV/rl-tracker-suite)
> - Original projects: [NeyorDEV/rl-session-stats](https://github.com/NeyorDEV/rl-session-stats) (this repository) and [NeyorDEV/lobbytracker](https://github.com/NeyorDEV/lobbytracker)

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

* **Multiple HUD Styles & Layouts**:
  * **Carte Horizontale**: Classic horizontal banner with high-resolution SVG rank badges, MMR readout, match delta indicator, and win/loss/streak counters.
  * **Colonne**: Modular cells with clean dividers and individual metrics.
  * **Carte Verticale**: Compact vertical side-widget format.
  * **Ligne**: Pure text & icon layout with shadow effects and zero bounding box.
* **MMR & Real-Time Sync**:
  * **Automatic Live MMR Sync**: Invisible, headless background sync via Rocket League Tracker Network (zero manual MMR entry needed).
  * **Accurate Official Brackets**: Exact MMR thresholds for 1v1, 2v2, and 3v3 (e.g. 1656 ELO = Grand Champion II Div. 3).
  * **Division Progress Bar**: Live progress percentage and exact MMR required to rank up to the next division.
  * **Floating Badges & Celebrations**: In-game floating MMR delta badges (`+11.2` / `-8.9`) and animated banners on Rank Up and Division Up.
  * **Focused Playlist Selector**: Focus on 1v1 (Duel), 2v2 (Doubles), or 3v3 (Standard) with instant tier & division recalculation.
  * **HD Modern Rank Icons**: Authentic transparent 256x256 post-F2P badges (including ruby GC1, GC2, GC3, and Supersonic Legend).
  * **Automatic Post-Match Synchronization**: Option to sync official tracker MMR automatically after each finished match.
* **Multi-Session History**:
  * **Day-by-Day Session Browser**: Browse previous game sessions day by day.
  * **Interactive MMR Progression Curve**: Vector SVG graph illustrating MMR evolution match after match.
  * **Match-by-Match Breakdown**: Detailed cards displaying playlist, result, MMR delta, and individual score/stats for each match.
  * **4 Live Summary Cards**: Record (W - L), Net MMR delta, Winrate, and Streak.
* **Session Goal Tracking**:
  * Configurable target MMR objective with a progress bar and exact remaining MMR required.
* **Deep Customization & Colors**:
  * **Fully Configurable Colors**: Customize every color of the overlay (Background, Primary Text, Secondary Text, Wins / Positive MMR, Losses / Negative MMR, Accent tags, and Goal/Streak).
  * **Opacity & Scale**: Adjust opacity (20% to 100%) and scale (60% to 200%).
  * **Screen Positioning**: Interactive 6-position screen grid with pixel-precise horizontal and vertical margin controls.
  * **Categorized Stat Selection**: Toggle individual stats cleanly categorized into *Rank & MMR*, *Session Results*, and *Match Events*.
* **Native Process Watcher & Autostart**:
  * **Ultra-Fast Game Watcher**: Native Win32 snapshot check puts overlay to sleep when Rocket League is closed and wakes up automatically upon game launch.
  * **Silent System Tray Autostart**: Windows startup option starts quietly in the background without UI freezes.
  * **Auto-Reset on Exit**: Option to automatically start fresh sessions whenever the application closes.
* **OBS & Streamlabs Ready**:
  * **Browser Source**: Stream real-time stats via the local web server (`http://localhost:5177/overlay`) powered by Server-Sent Events (SSE).
  * **Text Files**: Automatically generates and updates `.txt` files for text sources (`mmr.txt`, `mmr_change.txt`, `mmr_session.txt`, `rank.txt`, `division.txt`, `rank_full.txt`, `wins.txt`, `losses.txt`, `streak.txt`, etc.).
* **Ultra Lightweight**: Built on **Tauri 2 (Rust)** and vanilla web technologies with minimal CPU and RAM footprint.

---

## Download & Quick Start (Zero-Install)

You can download pre-compiled, ready-to-use Windows binaries directly from the **[Latest Release](https://github.com/NeyorDEV/rl-session-stats/releases/latest)**:

* **[RL Session Stats (portable).exe](https://github.com/NeyorDEV/rl-session-stats/releases/latest/download/RL.Session.Stats.portable.exe)** : Standalone portable executable (no installation required, simply double-click and run).
* **[RL Session Stats Setup (.exe)](https://github.com/NeyorDEV/rl-session-stats/releases/latest/download/RL.Session.Stats_1.3.0_x64-setup.exe)** : Standard Windows installer with desktop and Start menu shortcuts.

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
     - Click **Synchroniser le vrai MMR en direct** to instantly fetch your official MMR from Tracker Network. *(You can also toggle automatic synchronization after every match).*
4. **Launch Rocket League** and configure video settings:
   - Navigate to **Options -> Video -> Window Mode: Borderless**. This allows Windows to render transparent overlays on top of the game.
5. **Play!**
   - At the end of each game, your wins, losses, goals, saves, and MMR update and display in real time with floating delta badges and celebratory division rank up animations.
   - All session data is stored in your multi-session history so you can review previous days anytime.

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

# Build production bundle (.exe & setup to build-output/)
npm run build
```

---

## Code Signing & Integrity

Free code signing provided by [SignPath.io](https://signpath.io), certificate by [SignPath Foundation](https://signpath.org).

---

## Privacy Policy

RL Session Stats respects your privacy:
* **Zero telemetry & zero data collection**: All telemetry stays on your local machine.
* For more details, see [PRIVACY.md](PRIVACY.md).

---

## License

This project is licensed under the [MIT License](LICENSE) - see the LICENSE file for details.
