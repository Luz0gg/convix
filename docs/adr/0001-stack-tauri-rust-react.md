# ADR 0001 — Tauri 2 + Rust + React/TypeScript

- **Statut :** accepté (2026-11-06)

## Contexte
Logiciel de bureau Windows, léger, rapide, avec installeur, mises à jour automatiques, glisser-déposer natif et traitements lourds (encodage d'images, pilotage de FFmpeg). Le chef de projet ne code pas : la stack doit être robuste et bien connue de Claude.

## Options étudiées

| Option | Taille installeur | RAM au repos | Verdict |
|---|---|---|---|
| **Tauri 2** (Rust + WebView2 de Windows) | ~3-10 Mo | ~40-80 Mo | ✅ retenu |
| Electron (Node + Chromium embarqué) | ~120-200 Mo | ~150-400 Mo | trop lourd |
| Wails (Go + WebView) | ~8-30 Mo | faible | viable, écosystème plus petit, pas d'updater officiel aussi mûr |
| .NET WPF / WinUI | ~moyenne | moyenne | Windows uniquement, design moderne plus laborieux |
| Python + Qt | 50-150 Mo | moyenne | packaging fragile, lent pour l'image |

## Décision
- **Tauri 2** pour la fenêtre, l'installeur NSIS, l'updater officiel signé, l'instance unique, le glisser-déposer avec vrais chemins de fichiers.
- **Rust** pour tout le moteur : vitesse native, pas de ramasse-miettes, excellentes bibliothèques image (`image`, `oxipng`, `ravif`, `fast_image_resize`) et PDF (`lopdf`, `pdfium-render`). Le moteur est en crates séparées, testable sans interface.
- **React + TypeScript + Tailwind** pour l'interface : écosystème énorme, itérations rapides sur le design.

## Conséquences
- Windows 10/11 fournit déjà WebView2 → rien à embarquer.
- Première compilation Rust lente (quelques minutes), ensuite incrémentale.
- Portage macOS/Linux possible plus tard sans réécriture.
