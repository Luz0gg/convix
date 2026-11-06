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
- ✅ Conversions réelles testées avec FFmpeg 9.0.2 : les 7 formats de sortie, normalisation, 24 bits conservé, extraction depuis un MP4, vidéo sans son → message clair
- ⬜ Extraire l'audio sans réencoder (copie directe de la piste)
- ⬜ Garder la pochette des MP3/M4A

## Phase 3 — PDF ✅ (2026-11-06)
- ✅ Fusionner plusieurs PDF (ordre choisi dans l'interface)
- ✅ Séparer les pages (toutes ou une liste « 1-3, 5, 8- ») et extraire des pages dans un seul PDF
- ✅ Compresser : sans perte, équilibrée, forte (photos internes allégées) ; jamais plus gros que l'original
- ✅ Images → PDF (A4, Letter ou taille de l'image, marges, orientation auto ; JPEG intégrés sans perte, transparence gardée)
- ✅ PDF → PNG / JPEG (72, 150, 300 DPI) avec le moteur PDF **intégré à Windows** : pas de module PDFium à télécharger
- ✅ Miniature de la 1re page sur les cartes PDF
- ✅ Panneau « multi-fichiers » : ordre des fichiers (↑ ↓), résultat unique
- ✅ Clic droit sur les PDF (compresser, pages en PNG/JPEG, séparer) et « Convertir en PDF » sur les images : 10 images sélectionnées = 1 seul PDF
- ⬜ Glisser-déposer pour réordonner (aujourd'hui : flèches)
- ⬜ PDF protégés par mot de passe (aujourd'hui : message clair)

## Phase 4 — Livraison ✅ (2026-11-06)
- ✅ Mises à jour automatiques : `tauri-plugin-updater`, signature vérifiée, `latest.json` sur GitHub Releases, bandeau « Installer et redémarrer », réglage on/off + « Rechercher maintenant »
- ✅ Clé de signature (hors dépôt, à sauvegarder : voir `docs/RELEASE.md`)
- ✅ Script de release local `pnpm release -Version X.Y.Z [-Publish]`, guide `docs/RELEASE.md`
- ✅ Nom définitif **Convix** et icône (2 flèches en cercle, `assets/app-icon.svg`)
- ✅ CSP stricte (vérifiée dans la vraie app : aucune violation)
- ✅ Journal d'erreurs local (`%LOCALAPPDATA%\io.github.luz0gg.convix\logs\convix.log`) : échecs, plantages, erreurs de l'interface
- ✅ Section « À propos » : version, GitHub, signaler un problème, afficher le journal, crédits
- ✅ **v1.0.0 publiée** le 2026-11-06 : https://github.com/Luz0gg/convix/releases/tag/v1.0.0
- 🔜 Tester une vraie mise à jour automatique (publier une v1.0.1 sur un PC où la v1.0.0 est installée)
- 🎯 **V1.0** ✅

## Après la V1 (idées validées, ordre à décider)
- Presets « 1 clic » personnalisables + historique des conversions
- Vidéo : conversion, compression, vidéo → GIF, extraction d'images
- HEIC, RAW, SVG → PNG
- Upscale d'images (module Real-ESRGAN), suppression de fond
- Documents bureautiques (LibreOffice en module), OCR
- macOS / Linux
