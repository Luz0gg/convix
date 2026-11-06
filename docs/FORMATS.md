# Matrice des formats

Source de vérité : le code (`crates/convertio-*/src/lib.rs`, fonction `operations_for`). Ce tableau le résume.

## Images (intégré, aucun module) — ✅ fonctionnel

| Entrée ↓ / Sortie → | PNG | JPEG | WebP | AVIF | ICO | GIF | TIFF | BMP | TGA |
|---|---|---|---|---|---|---|---|---|---|
| PNG, JPEG, WebP, GIF, BMP, TIFF, ICO, TGA | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ |
| SVG (rendu vectoriel) | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ |
| HEIC (iPhone), AVIF | ✔ | ✔ | ✔ | ✔* | ✔ | ✔ | ✔ | ✔ | ✔ |

\* Lecture via les codecs de Windows. Si Windows ne les a pas, ConvertIO indique quoi installer (gratuit, Microsoft Store) : « Extensions d'image HEIF » + « Extensions vidéo HEVC » pour le HEIC, « Extension vidéo AV1 » pour l'AVIF.

### Options par format de sortie

| Sortie | Options | Moteur |
|---|---|---|
| PNG | optimisation sans perte (défaut : oui) | `image` + oxipng |
| JPEG | qualité (défaut 90), progressif, transparence → fond blanc | mozjpeg |
| WebP | sans perte oui/non, qualité (défaut 90) | libwebp |
| AVIF | qualité (défaut 75) | ravif (rav1e) |
| ICO | toutes tailles 16 → 256 (défaut) ou une seule taille, carré transparent | `image` |
| GIF, TIFF, BMP, TGA | — | `image` |
| depuis SVG | largeur de rendu (défaut : taille d'origine) | resvg |

### Autres actions

| Action | Formats | Résultat |
|---|---|---|
| Compresser sans perte | PNG | `photo-min.png`, pixels identiques (oxipng, effort rapide/équilibré/max) |
| Compresser plus léger | PNG (réduction à 256/128/64/32 couleurs), JPEG et WebP (qualité, défaut 75) | `photo-min.<ext>` |
| Redimensionner | tous sauf SVG et ICO | `photo-resized.<ext>`, largeur et/ou hauteur, proportions gardées ou non |

### Comportements garantis
- **Orientation EXIF appliquée** (photos de téléphone jamais tournées).
- **Profil couleur conservé** en PNG et JPEG (Display P3, Adobe RGB…).
- Métadonnées EXIF (GPS, appareil) **retirées** du résultat.
- GIF, WebP et APNG animés → GIF ou WebP : **toutes les images et leurs durées sont gardées**. Vers un format fixe (PNG, JPEG…) : première image.

## Audio (module `ffmpeg`) — ⬜ Phase 2

| Entrée | Sorties |
|---|---|
| MP3, WAV, FLAC, AAC, M4A, OGG, Opus | tous les autres |
| Vidéo (MP4, MOV, MKV, WebM, AVI) | extraction en MP3, WAV, FLAC, M4A |

Options : débit (96-320 kbps, formats avec perte), fréquence (source / 48 / 44,1 kHz), normalisation du volume.

## PDF — ⬜ Phase 3

| Action | Module |
|---|---|
| PDF → PNG / JPEG (une image par page, 72/150/300 DPI) | `pdfium` |
| Compresser | — |
| Fusionner plusieurs PDF | — |
| Séparer les pages | — |
| Images → PDF | — |
