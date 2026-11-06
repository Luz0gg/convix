# Convix

> Convertisseur de fichiers **local**, léger et extensible pour Windows : glisse un fichier, Convix te propose tout ce qu'il peut en faire.

- 🖼️ **Images** — PNG, JPEG, WebP, AVIF, GIF, BMP, TIFF, ICO, compression avec/sans perte, redimensionnement
- 🎧 **Audio** — MP3, WAV, FLAC, AAC, M4A, OGG, Opus, extraction audio depuis une vidéo
- 📄 **PDF** — fusion, séparation, compression, PDF ↔ images
- 🖱️ **Clic droit** dans l'Explorateur Windows
- 🔒 **100 % local** : aucun fichier n'est envoyé sur Internet
- 🧩 **Modules** téléchargés à la demande (FFmpeg, PDFium, plus tard upscale IA)

**Statut :** images complètes (Phase 1) ; audio via FFmpeg téléchargé au premier besoin (Phase 2) ; PDF ensuite (Phase 3). Voir la [roadmap](docs/ROADMAP.md).

## Documentation

| Document | Contenu |
|---|---|
| [docs/CADRAGE.md](docs/CADRAGE.md) | Vision, principes, décisions, périmètre V1, questions ouvertes |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Comment c'est construit |
| [docs/ROADMAP.md](docs/ROADMAP.md) | Phases et avancement |
| [docs/FORMATS.md](docs/FORMATS.md) | Matrice des conversions |
| [docs/adr/](docs/adr/) | Décisions techniques détaillées |

## Développement

Prérequis : [Rust](https://rustup.rs), [Node.js](https://nodejs.org) 20+, [pnpm](https://pnpm.io), WebView2 (déjà présent sur Windows 10/11).

```bash
pnpm install          # dépendances de l'interface
pnpm app:dev          # lance l'app en mode développement (rechargement à chaud)
cargo test --workspace  # tests du moteur
pnpm app:build        # installeur Windows → target/release/bundle/nsis/
```

## Licence

[MIT](LICENSE)
