# ADR 0002 — Outils lourds en modules téléchargés à la demande

- **Statut :** accepté (2026-11-06)

## Contexte
FFmpeg pèse ~100 Mo. L'embarquer gonflerait l'installeur et **chaque mise à jour** (l'updater retélécharge l'installeur complet).

## Décision
FFmpeg, PDFium et les futurs outils (upscale IA…) sont des **modules** : téléchargés une fois au premier besoin, vérifiés par SHA-256, rangés dans `%LOCALAPPDATA%`, mis à jour indépendamment via un manifeste JSON. Les images restent 100 % intégrées (Rust pur).

## Conséquences
- Installeur ~10 Mo, mises à jour de l'app de quelques Mo.
- Première conversion audio : un téléchargement d'une minute environ (avec progression, une seule fois).
- Pose la base du système de modules voulu pour la suite (upscale…).
- Prévoir à terme un « pack hors-ligne » pour installer sans Internet.
