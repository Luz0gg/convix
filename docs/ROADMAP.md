# Roadmap — Convix

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
- ✅ **Clic droit Explorateur** (HKCU, cascade « Convix › … ») + exécution directe de `--op` + nettoyage à la désinstallation
- ✅ Glisser un dossier entier (récursif, 5000 fichiers max)
- 🔜 Lecture HEIC (photos iPhone) et AVIF via les codecs Windows (WIC) — codé, **à tester sur une vraie photo iPhone** (impossible depuis la session de Claude)
- ✅ GIF / WebP / APNG animés : animation conservée vers GIF et WebP (option « Garder l'animation »)
- ✅ Miniatures des images dans les cartes (générées par le moteur, chargées seulement quand visibles)
- ✅ Clic droit **en arrière-plan** : pas de fenêtre, tous les fichiers sélectionnés regroupés, une notification à la fin, mêmes réglages que la dernière fois
- ⬜ Progression fine dans un fichier (utile pour les très grosses images / AVIF)
- ⬜ Conversion des profils couleur vers sRGB pour WebP/AVIF (aujourd'hui le profil est perdu)
- ⬜ Redimensionner un GIF animé (aujourd'hui : première image)

## Phase 2 — Modules & audio 🔜 (cœur livré le 2026-11-06)
- ✅ Gestionnaire de modules : manifeste intégré, téléchargement (TLS de Windows), SHA-256, extraction 7z, suppression des anciennes versions
- ✅ Module FFmpeg 9.0.2 (build gyan.dev, 34 Mo) — téléchargement réel testé
- ✅ Fenêtre « Télécharger FFmpeg ? » au premier besoin, puis l'action demandée se lance seule
- ✅ Section « Modules » dans les Réglages (installer, mettre à jour, supprimer)
- ✅ Audio : MP3, WAV, FLAC, M4A, AAC, OGG, Opus, AIFF ; débit, fréquence, précision 16/24 bits (« comme l'original » par défaut), normalisation du volume (−14 / −16 / −23 LUFS)
- ✅ Extraction audio depuis une vidéo (MP4, MOV, MKV, WebM, AVI)
- ✅ Progression réelle (lecture de `-progress`), annulation (le processus est arrêté)
- ✅ Clic droit sur les sons et vidéos (apparaît une fois FFmpeg installé)
- 🔜 **Conversion réelle à valider** : le code et les tests sont prêts, mais la session de Claude n'a pas eu le droit d'exécuter FFmpeg
- ⬜ Extraire l'audio sans réencoder (copie directe de la piste)
- ⬜ Garder la pochette des MP3/M4A

## Phase 3 — PDF ⬜
- ⬜ Fusion, séparation, compression (lopdf)
- ⬜ PDF → images (module PDFium), images → PDF
- ⬜ Interface « multi-fichiers » (ordre des pages/fichiers par glisser)

## Phase 4 — Livraison ⬜
- ⬜ Clé de signature des mises à jour + `tauri-plugin-updater` + `latest.json`
- ⬜ Script de release local (build + `gh release create`), `docs/RELEASE.md`
- ⬜ Icône et identité visuelle définitives (nom définitif choisi : **Convix**)
- ⬜ CSP stricte, journal d'erreurs local, écran « À propos »
- 🎯 **V1.0**

## Après la V1 (idées validées, ordre à décider)
- Presets « 1 clic » personnalisables + historique des conversions
- Vidéo : conversion, compression, vidéo → GIF, extraction d'images
- HEIC, RAW, SVG → PNG
- Upscale d'images (module Real-ESRGAN), suppression de fond
- Documents bureautiques (LibreOffice en module), OCR
- macOS / Linux
