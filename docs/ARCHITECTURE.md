# Architecture — ConvertIO

## Vue d'ensemble

```
┌──────────────────────────── ConvertIO.exe (Tauri) ────────────────────────────┐
│                                                                                │
│  Interface (React + TypeScript, src/)          Coquille (Rust, src-tauri/)     │
│  ─────────────────────────────────             ─────────────────────────────   │
│  • glisser-déposer, liste des fichiers  ──IPC──▶ commandes : inspect_files,     │
│  • actions générées depuis les         ◀──────   run_operation, module_status…  │
│    « OperationDescriptor »                     • instance unique, ligne de      │
│  • traductions FR/EN                             commande (clic droit)          │
│                                                         │                       │
│                                    ┌────────────────────┴───────────────┐       │
│                                    ▼                                    ▼       │
│                     Moteur (crates/convertio-core)      Modules (crates/convertio-modules)
│                     formats · détection · registre      téléchargement, checksum, │
│                     opérations · sortie · jobs           emplacement de ffmpeg…   │
│                          ▲        ▲        ▲                                     │
│              convertio-image  convertio-audio  convertio-pdf   (+ futurs crates) │
│              (Rust pur)       (pilote ffmpeg)   (lopdf + pdfium)                 │
└────────────────────────────────────────────────────────────────────────────────┘
```

## Arborescence

```
ConvertIO/
├── Cargo.toml                # workspace Rust (toutes les crates)
├── package.json              # interface (pnpm)
├── src/                      # INTERFACE React/TS
│   ├── App.tsx
│   ├── features/             # une fonctionnalité = un dossier (drop, files, settings, modules…)
│   ├── i18n/                 # fr.json, en.json + mini moteur de traduction
│   └── lib/                  # api.ts (seul point d'appel au Rust), types.ts (miroir des types Rust)
├── src-tauri/                # COQUILLE de l'application (fenêtre, commandes, installeur)
│   ├── tauri.conf.json       # nom, version, fenêtre, installeur NSIS
│   ├── capabilities/         # permissions accordées à l'interface
│   └── src/                  # lib.rs (démarrage), commands.rs, cli.rs, shell_integration.rs
├── crates/                   # MOTEUR, testable sans interface
│   ├── convertio-core/       # formats, opérations, registre, sortie, annulation/progression
│   ├── convertio-image/
│   ├── convertio-audio/
│   ├── convertio-pdf/
│   └── convertio-modules/
└── docs/                     # cadrage, architecture, roadmap, formats, décisions (adr/)
```

## Le concept central : les « opérations décrites »

Chaque convertisseur implémente le trait `Converter` (`crates/convertio-core/src/converter.rs`) :

```rust
fn id(&self) -> &'static str;                                   // "image"
fn operations_for(&self, input: Format) -> Vec<OperationDescriptor>;  // ce que je sais faire
fn run(&self, ctx: &JobContext) -> Result<Vec<PathBuf>>;       // le faire
```

Un `OperationDescriptor` décrit une action **sous forme de données** : id stable (`image.convert.png`), catégorie (convertir, compresser, modifier, extraire, combiner), format de sortie, modules requis, et **options** (curseur qualité, interrupteur sans perte, liste de débits…).

L'interface ne connaît rien aux images ou à l'audio : elle affiche les descripteurs et génère les réglages automatiquement. **Conséquences :**
- ajouter une conversion = modifier une crate Rust, zéro changement d'interface ;
- le registre agrège tous les convertisseurs : un PNG reçoit les conversions image **et** « images → PDF » du convertisseur PDF ;
- les ids stables servent aux presets, à l'historique et au menu clic droit.

## Détection des formats

`format::detect` lit les 8 premiers Ko et reconnaît la **signature** du fichier (crate `infer`) ; l'extension ne sert que de secours (SVG…). Un `.png` qui est en réalité un JPEG est donc traité comme un JPEG.

## Sortie des fichiers

`output::resolve` + `OutputPolicy` : à côté de l'original (défaut), dossier fixe, ou chemin choisi dans une boîte « Enregistrer sous ». Jamais d'écrasement (`photo (1).png`). Les conversions écrivent d'abord dans un fichier temporaire puis renomment (pas de fichier à moitié écrit si ça plante) — *à implémenter en Phase 1*.

## Modules externes

Outils lourds non inclus dans l'installeur, téléchargés au premier besoin :

| Module | Taille | Sert à | Licence |
|---|---|---|---|
| `ffmpeg` | ~80-130 Mo | audio (puis vidéo) | LGPL/GPL selon build |
| `pdfium` | ~5 Mo | rendu PDF → images | BSD/Apache |
| *(futur)* `realesrgan` | ~50 Mo | upscale d'images IA | BSD |

- Un **manifeste JSON** (hébergé dans le dépôt GitHub, branche `main`, `modules/manifest.json`) liste versions, URL, SHA-256 et taille par plateforme → on peut mettre à jour FFmpeg **sans publier une nouvelle version de l'app**.
- Téléchargement → vérification SHA-256 → extraction dans `%LOCALAPPDATA%\io.github.luz0gg.convertio\modules\<id>\<version>\` → enregistrement dans `installed.json`. Un fichier dont le hash ne correspond pas est supprimé, jamais exécuté.
- Une opération déclare `requires: ["ffmpeg"]` ; si le module manque, l'erreur `ModuleMissing` déclenche dans l'interface la proposition de téléchargement.
- Télécharger FFmpeg séparément (au lieu de le redistribuer dans notre installeur) simplifie aussi les obligations de licence.

## Clic droit dans l'Explorateur (V1)

- Menu en cascade **ConvertIO ›** écrit dans `HKCU\Software\Classes\SystemFileAssociations\.<ext>\shell\ConvertIO` (pas besoin de droits admin), généré depuis le registre : 4 à 6 actions les plus utiles par format + « Ouvrir dans ConvertIO ».
- Chaque entrée lance `ConvertIO.exe --op <operation-id> "<fichier>"`.
- **Instance unique** : si on lance l'action sur 20 fichiers, Windows démarre 20 processus ; le plugin `single-instance` les redirige tous vers la fenêtre déjà ouverte, qui les traite en lot.
- Sous Windows 11, ce menu « classique » apparaît dans « Afficher plus d'options ». Une entrée directement dans le nouveau menu demande un composant COM + un package MSIX « sparse » : phase ultérieure.
- Activable/désactivable dans les réglages ; nettoyé par le désinstalleur.
- **Mode arrière-plan** (`src-tauri/src/quiet.rs`, activé par défaut) : la fenêtre est créée cachée ; lancé avec `--op`, ConvertIO convertit sans l'afficher, regroupe les fichiers transmis par les autres lancements, utilise les dernières options choisies dans l'app (`options.json`), affiche une notification Windows puis se ferme 1,5 s après le dernier fichier. Les lancements reçus pendant le démarrage sont mis en file (`EARLY_LAUNCHES`) pour ne rien perdre.

## Mises à jour de l'application

- Plugin officiel `tauri-plugin-updater` + **GitHub Releases** (dépôt public) : l'app lit `latest.json` de la dernière release, télécharge l'installeur, vérifie sa **signature** (clé privée gardée hors du dépôt), installe.
- Pas de CI : on construit et publie en local avec un script (`pnpm app:build` puis `gh release create`). Voir `docs/RELEASE.md` *(Phase 4)*.

## Performance

- Conversions sur des threads dédiés (`spawn_blocking`, puis une file de jobs avec parallélisme = nb de cœurs).
- Profil release : LTO, `codegen-units = 1`, `strip`, `panic = abort` → binaire petit et rapide.
- Dépendances compilées optimisées même en dev (les codecs non optimisés sont 10-50× plus lents).
- Interface : React, pas de bibliothèque UI lourde, Tailwind (CSS généré minimal).

## Sécurité

- L'interface n'a accès qu'aux commandes déclarées (capabilities Tauri), pas au système de fichiers en direct.
- Modules et mises à jour vérifiés cryptographiquement.
- *(Phase 4)* Content-Security-Policy stricte dans `tauri.conf.json`.
