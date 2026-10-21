# RL Session Stats — contexte pour Claude Code

Remplaçant de RocketStats (plugin BakkesMod) pour Rocket League, depuis que Easy Anti-Cheat
empêche BakkesMod. Affiche les stats de session (victoires, défaites, streak, winrate, buts,
démos...) en overlay au-dessus du jeu et pour OBS.

## Règle absolue

**Aucune injection dans le jeu.** L'appli lit uniquement la Stats API officielle de Rocket League
(socket TCP local). Ne jamais proposer de hook, d'injection DLL ou de lecture mémoire du processus :
ça serait détecté par EAC et ferait bannir l'utilisateur.

## Stack

- Tauri 2 (Rust) + interface en HTML/CSS/JS vanilla, sans framework ni étape de build front.
- Le front est servi tel quel depuis `ui/` (`frontendDist` dans `tauri.conf.json`).
- Compilation Windows via GitHub Actions (`.github/workflows/build.yml`), qui accroche aussi
  les `.exe` aux Releases.

## Conventions

- Interface, messages, logs, noms de tests et README : **en anglais**.
- Commentaires du code : **en français**.
- L'utilisateur communique en français.

## Architecture

- `src-tauri/src/stats_client.rs` : connexion TCP à la Stats API (port 49123 par défaut),
  découpage du flux JSON (objets collés sans séparateur), reconnexion automatique.
  Le champ `Data` peut être un objet ou une chaîne JSON : les deux sont gérés.
- `src-tauri/src/session.rs` : logique de session (`Tracker`). Événements gérés :
  `MatchCreated`, `MatchInitialized`, `RoundStarted`, `UpdateState`, `StatfeedEvent`,
  `MatchEnded`, `MatchDestroyed`.
  - Joueur local identifié par `PrimaryId` (manuel ou détecté), sinon via la caméra
    (`Game.Target` quand `bHasTarget` et pas `bReplay`).
  - Partie quittée avant la fin = défaite (option). Si un gagnant était déjà connu
    (`Game.bHasWinner` + `Winner`), c'est lui qui compte.
  - Parties hors ligne (sans `MatchGuid`) ignorées par défaut.
  - Morts comptées via `StatfeedEvent` "Demolish" avec le joueur en `SecondaryTarget`.
- `src-tauri/src/rl_config.rs` : détection de l'install (manifestes Epic, bibliothèques Steam)
  et activation de la Stats API (`PacketSendRate`, `Port` dans la section
  `[TAGame.MatchStatsExporter_TA]`). Gère les .ini en UTF-8, UTF-8 BOM et UTF-16 LE.
  Sauvegarde `.bak` avant la première modification. Refuse si `RocketLeague.exe` tourne.
- `src-tauri/src/settings.rs` : paramètres en JSON camelCase, fusion partielle venant de l'UI.
- `src-tauri/src/web_server.rs` : serveur HTTP local (port 5177) pour la source navigateur OBS,
  mises à jour en direct par Server-Sent Events (`/events`).
- `src-tauri/src/main.rs` : fenêtres, icône de la zone de notification, raccourci global
  Ctrl+Shift+O, commandes appelées par l'interface, écriture des fichiers `.txt` pour OBS.
- `src-tauri/src/tests.rs` : 7 tests (`cargo test`), dont un scénario complet via un faux
  Rocket League sur un vrai socket.
- `ui/settings.html|css|js` : fenêtre des paramètres. `ui/bridge.js` fait le lien avec Tauri.
- `ui/overlay.html` : utilisé à la fois par la fenêtre overlay (mode Tauri, événements `overlay`)
  et par OBS (mode navigateur, SSE).

Données utilisateur : `%APPDATA%\com.rlsessionstats.app\` (`settings.json`, `session.json`,
`log.txt`, dossier `obs/`).

## État actuel

Vérifié (sous Linux) : compilation sans erreur ni warning, 7 tests OK, appli lancée avec un faux
Rocket League (comptage, overlay, fichiers OBS, journal).

**Pas encore vérifié** :
- Compilation sur Windows.
- Comportement avec le vrai jeu (le format des événements vient de la doc officielle,
  pas de données réelles observées).
- Transparence de l'overlay au-dessus du jeu en fenêtré sans bordure.
- Détection automatique du dossier du jeu sur une vraie install Epic/Steam.

## Prochaines étapes

1. Compiler en local : `npm install` puis `npx tauri build` (ou `npx tauri dev` pour itérer).
2. Tester avec le vrai jeu et lire `log.txt` : on doit y voir « Connected to Rocket League »,
   puis « Tracking player: ... », puis « Win » / « Loss » à chaque fin de match.
3. Corriger ce qui ne correspond pas aux vraies données de la Stats API.
4. Publier sur GitHub (compte NeyorDEV) : dépôt privé et pre-release d'abord,
   public une fois validé.

## Limites connues

- Pas de MMR ni de rang : la Stats API ne les fournit pas.
- Pas d'overlay en plein écran exclusif (fenêtré sans bordure obligatoire).
- Nécessite WebView2 (présent sur Windows 11 et la plupart des Windows 10 à jour).
