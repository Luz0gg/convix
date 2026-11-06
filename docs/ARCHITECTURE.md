# Architecture — Convix

## Vue d'ensemble

```
┌──────────────────────────── Convix.exe (Tauri) ────────────────────────────┐
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
│                     Moteur (crates/convix-core)      Modules (crates/convix-modules)
│                     formats · détection · registre      téléchargement, checksum, │
│                     opérations · sortie · jobs           emplacement de ffmpeg…   │
│                          ▲        ▲        ▲                                     │
│              convix-image  convix-audio  convix-pdf   (+ futurs crates) │
│              (Rust pur)       (pilote ffmpeg)   (lopdf + Windows)                │
└────────────────────────────────────────────────────────────────────────────────┘
```

## Arborescence

```
Convix/
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
│   ├── convix-core/       # formats, opérations, registre, sortie, annulation/progression
│   ├── convix-image/
│   ├── convix-audio/
│   ├── convix-pdf/
│   └── convix-modules/
└── docs/                     # cadrage, architecture, roadmap, formats, décisions (adr/)
```

## Le concept central : les « opérations décrites »

Chaque convertisseur implémente le trait `Converter` (`crates/convix-core/src/converter.rs`) :

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

`output::resolve` + `OutputPolicy` : à côté de l'original (défaut), dossier fixe, ou chemin choisi dans une boîte « Enregistrer sous ». Jamais d'écrasement (`photo (1).png`). Les conversions écrivent d'abord dans un fichier temporaire (`.part`) puis renomment : pas de fichier à moitié écrit si ça plante.

## Modules externes

Outils lourds non inclus dans l'installeur, téléchargés au premier besoin :

| Module | Taille | Sert à | Licence |
|---|---|---|---|
| `ffmpeg` | 34 Mo à télécharger (archive 7z), 105 Mo installé (`ffmpeg.exe` seul) | audio (puis vidéo) | GPL (build gyan.dev, lien officiel de ffmpeg.org) |
| *(futur)* `realesrgan` | ~50 Mo | upscale d'images IA | BSD |

- Un **manifeste JSON** compilé dans l'app (`crates/convix-modules/manifest.json`) liste, par plateforme : version, URL, SHA-256, taille et fichiers à garder. Pour passer à une nouvelle version de FFmpeg, on modifie ce fichier : elle arrive avec la mise à jour automatique de Convix. *(Plus tard, si besoin : un manifeste distant.)*
- Étapes de l'installation :
  1. Téléchargement avec `ureq` et le TLS de Windows (certificats du système, rien à compiler).
  2. Vérification de la taille et de l'empreinte SHA-256.
  3. Extraction 7z (`sevenz-rust2`, Rust pur) des **seuls** fichiers listés, dans `%LOCALAPPDATA%\io.github.luz0gg.convix\modules\<id>\<version>\`.
  4. Enregistrement dans `installed.json` (écriture atomique), puis suppression des anciennes versions.
- Un fichier dont l'empreinte ne correspond pas est supprimé, jamais extrait ni exécuté.
- Une opération déclare `requires: ["ffmpeg"]`. Tant que le module manque, ses boutons portent ⬇. Au clic, l'interface propose le téléchargement puis lance l'action. Côté Rust, `jobs.rs` vérifie aussi, avant de réserver le nom de sortie (`errorCode: "moduleMissing:ffmpeg"`).
- FFmpeg tourne sans fenêtre console (`CREATE_NO_WINDOW`) et lit `-progress pipe:1` pour afficher la vraie progression. Annuler arrête le processus. Il écrit dans `<sortie>.part`, puis renomme : jamais de fichier à moitié écrit.
- Télécharger FFmpeg séparément (au lieu de le redistribuer dans notre installeur) simplifie aussi les obligations de licence.

## Clic droit dans l'Explorateur (V1)

- Menu en cascade **Convix ›** (images, sons, vidéos ; les entrées audio n'apparaissent qu'une fois FFmpeg installé) écrit dans `HKCU\Software\Classes\SystemFileAssociations\.<ext>\shell\Convix` (pas besoin de droits admin), généré depuis le registre : 4 à 6 actions les plus utiles par format + « Ouvrir dans Convix ».
- Chaque entrée lance `Convix.exe --op <operation-id> "<fichier>"`.
- **Instance unique** : si on lance l'action sur 20 fichiers, Windows démarre 20 processus ; le plugin `single-instance` les redirige tous vers la fenêtre déjà ouverte, qui les traite en lot.
- Sous Windows 11, ce menu « classique » apparaît dans « Afficher plus d'options ». Une entrée directement dans le nouveau menu demande un composant COM + un package MSIX « sparse » : phase ultérieure.
- Activable/désactivable dans les réglages ; nettoyé par le désinstalleur.
- **Mode arrière-plan** (`src-tauri/src/quiet.rs`, activé par défaut) : la fenêtre est créée cachée ; lancé avec `--op`, Convix convertit sans l'afficher, regroupe les fichiers transmis par les autres lancements, utilise les dernières options choisies dans l'app (`options.json`), affiche une notification Windows puis se ferme 1,5 s après le dernier fichier. Les lancements reçus pendant le démarrage sont mis en file (`EARLY_LAUNCHES`) pour ne rien perdre.

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
