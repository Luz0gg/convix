# Roadmap — ConvertIO

Chaque phase se termine par une version utilisable. ✅ fait · 🔜 en cours · ⬜ à faire

## Phase 0 — Cadrage & squelette ✅ (2026-11-06)
- ✅ Choix techniques, décisions, documentation (`docs/`)
- ✅ Workspace Rust : moteur, registre, formats, opérations décrites, politique de sortie
- ✅ Convertisseurs image / audio / PDF déclarés (opérations listées, exécution à venir)
- ✅ Interface : glisser-déposer natif, cartes fichiers avec actions, FR/EN, thème sombre
- ✅ Ligne de commande + instance unique (prépare le clic droit)

## Phase 1 — Images, de bout en bout 🔜 (cœur livré le 2026-11-06)
- ✅ Conversions PNG, JPEG, WebP, GIF, BMP, TIFF, ICO, TGA, SVG → PNG, JPEG, WebP, AVIF, ICO, GIF, TIFF, BMP, TGA
- ✅ Orientation EXIF appliquée, profil couleur conservé (PNG, JPEG), transparence → fond blanc en JPEG
- ✅ Compression sans perte (oxipng) et « plus léger » (palette PNG, mozjpeg, WebP)
- ✅ Redimensionnement (proportions gardées ou non)
- ✅ ICO multi-tailles (16 → 256), SVG rendu à la largeur voulue
- ✅ Panneau d'options généré depuis `OptionSpec`, options mémorisées par action
- ✅ File de jobs : lot parallèle (1 fichier par cœur), statut par fichier, annulation, écriture atomique, noms jamais écrasés
- ✅ Résultat : taille avant/après, gain en %, bouton « Afficher »
- ✅ Actions « Pour tous les fichiers » quand plusieurs fichiers sont déposés
- ✅ Réglages persistants : dossier de sortie (à côté / fixe / demander), langue, thème
- ✅ **Clic droit Explorateur** (HKCU, cascade « ConvertIO › … ») + exécution directe de `--op` + nettoyage à la désinstallation
- ✅ Glisser un dossier entier (récursif, 5000 fichiers max)
- ⬜ Lecture AVIF et HEIC (photos iPhone) — décodeurs natifs à intégrer
- ⬜ GIF / WebP animés (aujourd'hui : première image seulement)
- ⬜ Aperçu miniature des images dans les cartes
- ⬜ Progression fine dans un fichier (utile pour les très grosses images / AVIF)
- ⬜ Conversion des profils couleur vers sRGB pour WebP/AVIF (aujourd'hui le profil est perdu)
- ⬜ Mode « silencieux » du clic droit (convertir sans afficher la fenêtre, juste une notification)

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
- Documents bureautiques (LibreOffice en module), OCR
- macOS / Linux
