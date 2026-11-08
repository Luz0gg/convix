# Matrice des formats

Source de vérité : le code (`crates/convix-*/src/lib.rs`, fonction `operations_for`). Ce tableau le résume.

## Images (intégré, aucun module) — ✅ fonctionnel

| Entrée ↓ / Sortie → | PNG | JPEG | WebP | AVIF | ICO | GIF | TIFF | BMP | TGA |
|---|---|---|---|---|---|---|---|---|---|
| PNG, JPEG, WebP, GIF, BMP, TIFF, ICO, TGA | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ |
| SVG (rendu vectoriel) | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ |
| HEIC (iPhone), AVIF | ✔ | ✔ | ✔ | ✔* | ✔ | ✔ | ✔ | ✔ | ✔ |

\* Lecture via les codecs de Windows. Si Windows ne les a pas, Convix indique quoi installer (gratuit, Microsoft Store) : « Extensions d'image HEIF » + « Extensions vidéo HEVC » pour le HEIC, « Extension vidéo AV1 » pour l'AVIF.

### Options par format de sortie

| Sortie | Options | Moteur |
|---|---|---|
| PNG | optimisation sans perte (défaut : oui), côté max | `image` + oxipng |
| JPEG | qualité (défaut 90), progressif, transparence → fond blanc, côté max | mozjpeg |
| WebP | sans perte oui/non, qualité (défaut 90), côté max | libwebp |
| AVIF | qualité (défaut 75), côté max | ravif (rav1e) |
| ICO | toutes tailles 16 → 256 (défaut) ou une seule taille, carré transparent | `image` |
| GIF, TIFF, BMP, TGA | — | `image` |
| depuis SVG | largeur de rendu (défaut : taille d'origine) | resvg |

### Autres actions

| Action | Formats | Résultat |
|---|---|---|
| Compresser sans perte | PNG | `photo-min.png`, pixels identiques (oxipng, effort rapide/équilibré/max) |
| Compresser plus léger | PNG (réduction à 256/128/64/32 couleurs), JPEG et WebP (qualité, défaut 75) | `photo-min.<ext>` |
| Redimensionner | tous sauf SVG et ICO | `photo-resized.<ext>`, largeur et/ou hauteur (proportions gardées ou non), **ou un pourcentage** |

« Côté max » : le côté le plus long est ramené à N pixels s'il dépasse (jamais agrandi). Pratique pour convertir un dossier de photos pour le web en un seul lancement.

### Comportements garantis
- **Orientation EXIF appliquée** (photos de téléphone jamais tournées).
- **Profil couleur conservé** en PNG et JPEG (Display P3, Adobe RGB…).
- Métadonnées EXIF (GPS, appareil) **retirées** du résultat.
- GIF, WebP et APNG animés → GIF ou WebP : **toutes les images et leurs durées sont gardées**. Vers un format fixe (PNG, JPEG…) : première image.

## Audio (module `ffmpeg`, téléchargé au premier besoin) — 🔜 Phase 2

| Entrée | Sorties |
|---|---|
| MP3, WAV, FLAC, AAC, M4A, OGG, Opus, AIFF | tous les autres |
| Vidéo (MP4, MOV, MKV, WebM, AVI) | extraction en MP3, WAV, FLAC, M4A |

| Sortie | Encodeur | Options |
|---|---|---|
| MP3 | LAME | débit (défaut 320 kbps) |
| M4A, AAC | AAC natif FFmpeg | débit (défaut 192 kbps) |
| OGG | Vorbis | débit (défaut 192 kbps) |
| Opus | libopus | débit (défaut 128 kbps), toujours 48 kHz |
| WAV, AIFF | PCM | précision : comme l'original (16, 24, 32 bits ou flottant) / 16 / 24 bits |
| FLAC | FLAC niveau 8 | précision : comme l'original / 16 / 24 bits |

Pour toutes : fréquence (comme l'original / 48 / 44,1 kHz) et volume (inchangé, normalisé −14 LUFS streaming, −16 LUFS podcast, −23 LUFS TV). La normalisation garde la fréquence d'origine. Les tags (artiste, titre…) sont conservés, la vidéo et la pochette sont retirées. Seule la première piste audio est prise.

## Vidéo (module `ffmpeg`) — nouveau

| Action | Entrées | Résultat | Options |
|---|---|---|---|
| En MP4 | MOV, MKV, WebM, AVI | H.264 + AAC, lecture rapide sur le web (`faststart`) | qualité haute / équilibrée / fichier léger, définition max (4K, 1080p, 720p, 480p) |
| En WebM | MP4, MOV, MKV, AVI | VP9 + Opus | idem |
| En GIF | toutes | palette optimisée, en boucle, sans son | largeur (320 → 800 px), 10 / 15 / 24 images/s |
| Compresser la vidéo | toutes | `clip-min.mp4`, H.264 ; 1080p max par défaut | qualité, définition max |

La définition max s'applique au **petit côté** (une vidéo de téléphone en portrait 1080×1920 limitée à 720p devient 720×1280) et n'agrandit jamais. Première piste vidéo et première piste audio ; sous-titres retirés.

## PDF (intégré, aucun module) — ✅ Phase 3

| Action | Résultat | Moteur |
|---|---|---|
| PDF → PNG / JPEG | une image par page : `doc-p01.png`, `doc-p02.png`… ; 72 / 150 / 300 DPI ; pages au choix | moteur PDF de Windows (`Windows.Data.Pdf`) |
| Compresser | `doc-min.pdf` ; sans perte (nettoyage + flux compressés) / équilibrée (photos ≤ 2200 px, JPEG 75) / forte (≤ 1400 px, JPEG 60) ; jamais plus gros que l'original | lopdf + mozjpeg |
| Séparer les pages | un PDF par page : `doc-p1.pdf`… ; toutes ou une liste | lopdf |
| Extraire / réordonner des pages | `doc-extrait.pdf` avec les pages choisies, **dans l'ordre tapé** (`3, 1-2` réordonne) | lopdf |
| Faire pivoter des pages | `doc-rot.pdf` ; ↻ 90°, ↺ 90° ou 180°, toutes les pages ou une liste ; sans perte | lopdf |
| Supprimer des pages | `doc-modif.pdf` sans les pages indiquées | lopdf |
| Fusionner plusieurs PDF | `<premier>-fusion.pdf`, dans l'ordre choisi ; les signets sont retirés | lopdf |
| Images → PDF | une page par image ; A4 / Letter / taille de l'image, marges 1 cm ; JPEG intégrés tels quels, transparence gardée | lopdf |

Listes de pages : `1-3, 5, 8-` (vide = toutes). Les listes de pages ne sont pas mémorisées d'une fois sur l'autre. PDF protégé par mot de passe : message clair, pas de traitement.
