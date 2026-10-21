const fs = require('fs');
const path = require('path');

const rootDir = path.resolve(__dirname, '..');
const outDir = path.join(rootDir, 'build-output', 'RL-Session-Stats-Windows');
const targetRelease = path.join(rootDir, 'src-tauri', 'target', 'release');
const nsisDir = path.join(targetRelease, 'bundle', 'nsis');

if (!fs.existsSync(outDir)) {
  fs.mkdirSync(outDir, { recursive: true });
}

// Nettoyer les anciens exécutables dans build-output
for (const file of fs.readdirSync(outDir)) {
  if (file.endsWith('.exe')) {
    fs.unlinkSync(path.join(outDir, file));
  }
}

// Copier le binaire portable
const portableSrc = path.join(targetRelease, 'rl-session-stats.exe');
if (fs.existsSync(portableSrc)) {
  const portableDest = path.join(outDir, 'RL Session Stats (portable).exe');
  fs.copyFileSync(portableSrc, portableDest);
  console.log(`Copie portable : ${portableDest}`);
}

// Copier l'installeur NSIS
if (fs.existsSync(nsisDir)) {
  for (const file of fs.readdirSync(nsisDir)) {
    if (file.endsWith('.exe')) {
      const src = path.join(nsisDir, file);
      const dest = path.join(outDir, file);
      fs.copyFileSync(src, dest);
      console.log(`Copie setup NSIS : ${dest}`);
    }
  }
}

console.log(`Fichiers de build synchronises avec succes dans build-output/RL-Session-Stats-Windows !`);
