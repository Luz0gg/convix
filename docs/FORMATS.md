# Matrice des formats

Source de vérité : le code (`crates/convertio-*/src/lib.rs`, fonction `operations_for`). Ce tableau résume l'objectif V1.

## Images (intégré, aucun module)

| Entrée ↓ / Sortie → | PNG | JPEG | WebP | AVIF | GIF | BMP | TIFF | ICO | PDF |
|---|---|---|---|---|---|---|---|---|---|
| PNG, JPEG, WebP, AVIF, GIF, BMP, TIFF, ICO | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ (images → PDF) |
| HEIC | plus tard (libheif) |
| SVG | plus tard (resvg) |

Actions en plus : compression sans perte, compression « plus léger », redimensionnement.

| Sortie | Options |
|---|---|
| JPEG | qualité (défaut 90) |
| WebP | sans perte oui/non, qualité (défaut 90) |
| AVIF | qualité (défaut 75) |
| PNG | optimisation sans perte (défaut oui) |
| ICO | taille 256/128/64/32 |

## Audio (module `ffmpeg`)

| Entrée | Sorties |
|---|---|
| MP3, WAV, FLAC, AAC, M4A, OGG, Opus | tous les autres |
| Vidéo (MP4, MOV, MKV, WebM, AVI) | extraction en MP3, WAV, FLAC, M4A |

Options : débit (96-320 kbps, formats avec perte), fréquence (source / 48 / 44,1 kHz), normalisation du volume.

## PDF

| Action | Module |
|---|---|
| PDF → PNG / JPEG (une image par page, 72/150/300 DPI) | `pdfium` |
| Compresser | — |
| Fusionner plusieurs PDF | — |
| Séparer les pages | — |
| Images → PDF | — |
