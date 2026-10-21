# RL Session Stats

Remplaçant de RocketStats pour Rocket League depuis l'arrivée d'Easy Anti-Cheat.
Affiche victoires, défaites, streak, winrate, buts, démos… de ta session, en overlay
par-dessus le jeu et/ou dans OBS.

Aucune injection dans le jeu : l'appli lit uniquement la **Stats API officielle** de
Rocket League. Tu peux jouer en ligne normalement, avec EAC activé.

---

## 1. Obtenir le fichier .exe (une seule fois, sans rien installer)

La compilation se fait gratuitement sur les serveurs de GitHub.

1. Crée un compte sur [github.com](https://github.com) si tu n'en as pas.
2. Clique sur **+** (en haut à droite) → **New repository**.
   Donne-lui un nom (par ex. `rl-session-stats`), laisse le reste par défaut, puis **Create repository**.
3. Sur la page du dépôt, clique sur le lien **uploading an existing file**.
4. Glisse **tout le contenu** du dossier dézippé (les dossiers `.github`, `src-tauri`, `ui`
   et les fichiers `package.json`, `.gitignore`, `LISEZ-MOI.md`), puis **Commit changes**.
   > Vérifie que le dossier `.github` est bien envoyé : c'est lui qui déclenche la compilation.
5. Ouvre l'onglet **Actions**. La compilation « Compiler pour Windows » démarre toute seule
   (compte environ 10 minutes la première fois). Si rien ne démarre, clique dessus puis
   **Run workflow**.
6. Quand la pastille est verte ✅, clique sur la compilation, descends jusqu'à **Artifacts**
   et télécharge **RL-Session-Stats-Windows**. Le zip contient :
   - `RL Session Stats (portable).exe` : à lancer directement, sans installation ;
   - `RL Session Stats_1.0.0_x64-setup.exe` : un installateur classique (raccourci dans le menu Démarrer).

> **Avertissement Windows « a protégé votre ordinateur »** : normal pour une appli non signée.
> Clique sur **Informations complémentaires** → **Exécuter quand même**.

---

## 2. Première utilisation

1. **Ferme Rocket League.**
2. Lance l'appli. Dans la section **Stats API de Rocket League**, clique sur
   **Activer la Stats API**. L'appli trouve le jeu (Epic ou Steam) et règle le fichier
   à ta place (une sauvegarde `.bak` est gardée).
   Si le jeu n'est pas trouvé : **Choisir le dossier…** et sélectionne le dossier `rocketleague`.
3. Dans Rocket League : **Paramètres → Vidéo → Mode d'affichage : Fenêtré sans bordure**
   (nécessaire pour voir l'overlay par-dessus le jeu).
4. Joue : ton joueur est reconnu tout seul au premier match.

---

## 3. Au quotidien

- **Ctrl + Maj + O** : afficher / masquer l'overlay.
- **Réinitialiser la session** : bouton en bas à gauche de la fenêtre (deux clics pour confirmer),
  ou clic droit sur l'icône près de l'horloge.
- Fermer la fenêtre laisse l'overlay actif. Pour quitter complètement : bouton
  **Quitter l'application** ou clic droit sur l'icône → **Quitter**.

## 4. Stream (OBS / Streamlabs)

- **Source navigateur** : copie l'adresse affichée dans la section OBS de l'appli
  (par défaut `http://localhost:5177/overlay`).
- **Sources texte** : bouton **Ouvrir le dossier** → un fichier `.txt` par stat
  (`wins.txt`, `losses.txt`, `streak.txt`…), comme avec RocketStats.

## 5. Si plus rien n'est compté

Une mise à jour de Rocket League peut désactiver la Stats API.
Ferme le jeu, ouvre l'appli et clique sur **Réappliquer le réglage**.

Un journal est disponible dans `%APPDATA%\fr.rlsessionstats.app\journal.txt`
(il indique quand le jeu est connecté, le joueur reconnu et chaque partie comptée).

## Limites

- Le **MMR** et le rang ne sont pas disponibles : la Stats API officielle ne les fournit pas.
- L'overlay ne s'affiche pas en **plein écran exclusif** (il faut le mode fenêtré sans bordure).
- Nécessite WebView2, déjà présent sur Windows 11 et sur la plupart des Windows 10 à jour.
