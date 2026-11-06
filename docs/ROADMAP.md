# Roadmap — ConvertIO

Chaque phase se termine par une version utilisable. ✅ fait · 🔜 en cours · ⬜ à faire

## Phase 0 — Cadrage & squelette ✅ (2026-11-06)
- ✅ Choix techniques, décisions, documentation (`docs/`)
- ✅ Workspace Rust : moteur, registre, formats, opérations décrites, politique de sortie
- ✅ Convertisseurs image / audio / PDF déclarés (opérations listées, exécution à venir)
- ✅ Interface : glisser-déposer natif, cartes fichiers avec actions, FR/EN, thème sombre
- ✅ Ligne de commande + instance unique (prépare le clic droit)

## Phase 1 — Images, de bout en bout ⬜
- ⬜ Conversions entre PNG, JPEG, WebP, AVIF, GIF, BMP, TIFF, ICO (`image`, `webp`, `ravif`)
- ⬜ Compression sans perte (oxipng) et « plus léger » (mozjpeg / WebP lossy)
- ⬜ Redimensionnement (fast_image_resize)
- ⬜ Panneau d'options généré depuis `OptionSpec` (curseurs, interrupteurs, listes)
- ⬜ File de jobs : lot, progression en temps réel, annulation, écriture atomique
- ⬜ Résultat : « Ouvrir le dossier », taille avant/après, gain en %
- ⬜ Réglages persistants : dossier de sortie (à côté / fixe / demander), langue, thème
- ⬜ **Clic droit Explorateur** (HKCU) + exécution directe de `--op`
- ⬜ Glisser un dossier entier

## Phase 2 — Modules & audio ⬜
- ⬜ Gestionnaire de modules : manifeste, téléchargement, SHA-256, extraction, mise à jour
- ⬜ Écran « Modules » (installé / à télécharger / mise à jour dispo)
- ⬜ Audio via FFmpeg : MP3, WAV, FLAC, AAC, M4A, OGG, Opus, débit, fréquence, normalisation (loudnorm)
- ⬜ Extraction audio depuis une vidéo
- ⬜ Progression FFmpeg réelle (lecture de `-progress`)

## Phase 3 — PDF ⬜
- ⬜ Fusion, séparation, compression (lopdf)
- ⬜ PDF → images (module PDFium), images → PDF
- ⬜ Interface « multi-fichiers » (ordre des pages/fichiers par glisser)

## Phase 4 — Livraison ⬜
- ⬜ Clé de signature des mises à jour + `tauri-plugin-updater` + `latest.json`
- ⬜ Script de release local (build + `gh release create`), `docs/RELEASE.md`
- ⬜ Icône et identité visuelle définitives, nom définitif
- ⬜ CSP stricte, journal d'erreurs local, écran « À propos »
- 🎯 **V1.0**

## Après la V1 (idées validées, ordre à décider)
- Presets « 1 clic » personnalisables + historique des conversions
- Vidéo : conversion, compression, vidéo → GIF, extraction d'images
- HEIC, RAW, SVG → PNG
- Upscale d'images (module Real-ESRGAN), suppression de fond
- Dossier surveillé (conversion automatique)
- Menu clic droit Windows 11 natif
- Documents bureautiques (LibreOffice en module), OCR
- macOS / Linux
