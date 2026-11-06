# Cadrage — ConvertIO

> Document de référence du projet. Toute décision structurante est notée ici (ou dans `docs/adr/`).
> Dernière mise à jour : 2026-11-06.

## 1. Vision

Un **logiciel Windows léger** qui convertit n'importe quel fichier en un autre, **en local** (rien n'est envoyé sur Internet), en un geste :

1. J'ouvre ConvertIO (ou je fais clic droit sur un fichier dans l'Explorateur).
2. Je glisse-dépose un ou plusieurs fichiers.
3. ConvertIO détecte le type réel du fichier et m'affiche **tout ce qu'il peut en faire** : convertir en X, compresser avec/sans perte, redimensionner, extraire…
4. Un clic → le résultat apparaît **à côté de l'original**.

À terme : une boîte à outils fichiers complète, extensible par **modules** (upscale d'images, etc.).

**Utilisateur cible n°1 :** un monteur vidéo qui jongle en permanence avec des formats (WebP → PNG, WAV → MP3, PDF → images…) et veut arrêter de passer par des sites web.

## 2. Principes non négociables

| Principe | Concrètement |
|---|---|
| **Local et privé** | Aucune donnée envoyée. Seuls téléchargements : modules et mises à jour, depuis des sources vérifiées (checksum + signature). |
| **Léger** | Installeur ~10 Mo. Les outils lourds (FFmpeg…) sont des modules téléchargés au premier besoin. |
| **Simple** | Glisser → choisir → c'est fait. Les options avancées existent mais sont repliées. |
| **Extensible** | Ajouter une conversion = du code moteur uniquement ; l'interface s'adapte toute seule. |
| **Jamais destructif** | On n'écrase jamais un fichier : `photo.png` existe → `photo (1).png`. |
| **Rapide** | Moteur natif (Rust), conversions en parallèle, interface jamais bloquée. |

## 3. Décisions prises (2026-11-06)

| Sujet | Décision | Pourquoi |
|---|---|---|
| Plateforme | **Windows d'abord**, code portable (macOS/Linux possibles plus tard) | Besoin de l'utilisateur ; Tauri est multiplateforme sans surcoût. |
| Technologie | **Tauri 2 + Rust (moteur) + React/TypeScript (interface)** | Voir [ADR 0001](adr/0001-stack-tauri-rust-react.md). |
| FFmpeg | **Module téléchargé à la demande**, pas dans l'installeur | Installeur léger, mises à jour minuscules. [ADR 0002](adr/0002-modules-a-la-demande.md). |
| Dépôt GitHub | **Public**, licence **MIT** | Mises à jour automatiques via GitHub Releases sans serveur. |
| CI GitHub Actions | **Aucune** pour l'instant | Ne pas consommer de crédits. Builds et releases faits en local. |
| Langues | **Français (défaut) + anglais** dès le départ | Ajouter une langue = un fichier JSON. |
| Sortie par défaut | **À côté de l'original** ; réglages : dossier fixe ou « demander à chaque fois » | Le plus naturel pour un monteur. |
| Clic droit Explorateur | **Dès la V1** | Gain de temps énorme au quotidien. [ADR 0003](adr/0003-menu-contextuel.md). |
| Nom | **ConvertIO = nom de code**, centralisé pour renommer facilement | « Convertio » est déjà un service connu (convertio.co) : à changer avant toute diffusion publique large. |
| Priorité formats | **Images, audio, PDF** d'abord. **Vidéo plus tard** | La vidéo a déjà de bons outils (Shutter Encoder) ; l'extraction audio d'une vidéo reste en V1, c'est presque gratuit. |
| Thème | **Sombre par défaut**, suit le thème clair du système | Cohérent avec les logiciels de montage. |

## 4. Périmètre V1 (« MVP utile au quotidien »)

**Dans la V1 :**
- Glisser-déposer (un ou plusieurs fichiers, dossiers plus tard), détection du vrai format.
- **Images** : PNG, JPEG, WebP, AVIF, GIF, BMP, TIFF, ICO ↔ entre eux ; compression sans perte / plus léger ; redimensionnement.
- **Audio** (module FFmpeg) : MP3, WAV, FLAC, AAC, M4A, OGG, Opus ↔ entre eux ; débit, fréquence, normalisation du volume ; **extraire l'audio d'une vidéo**.
- **PDF** : fusion, séparation, compression, PDF → images (module PDFium), images → PDF.
- Traitement par lot + barre de progression + annulation.
- Réglages : dossier de sortie, langue, thème, modules installés.
- Clic droit dans l'Explorateur.
- Installeur Windows (NSIS, sans droits admin) + mises à jour automatiques.

**Hors V1 (plus tard) :** vidéo complète (codecs, compression, GIF), HEIC/RAW, SVG, documents bureautiques (Word/Excel → PDF), OCR, upscale IA, suppression de fond, presets personnalisés partagés, macOS/Linux.

## 5. Questions encore ouvertes

- [ ] **Nom définitif** (et vérification de disponibilité : domaine, GitHub, marque).
- [ ] **Logo / identité visuelle.**
- [ ] **Presets** : quels raccourcis « 1 clic » veux-tu en premier ? (ex. « WebP → PNG », « WAV → MP3 320 »).
- [ ] **Signature de code Windows** (certificat payant, évite l'alerte SmartScreen) : maintenant ou plus tard ?
- [ ] **Conversion automatique d'un dossier surveillé** (ex. tout ce qui arrive dans `Téléchargements\*.webp` → PNG) : utile pour toi ?
