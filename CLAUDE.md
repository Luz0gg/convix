# CLAUDE.md — instructions pour les sessions Claude sur Convix

## Rôles
- L'utilisateur est **chef de projet**, ne code pas. Il décide ; Claude conçoit, code, explique en français simple.
- Langue : **français** pour les échanges et la doc. Code, identifiants et commentaires de code en **anglais**.
- Avant une décision produit structurante : poser la question (options + recommandation). Choix purement techniques : Claude tranche et l'explique.

## Règles impératives
- **Pas de CI GitHub Actions** (pas de `.github/workflows/`) — économie de crédits. Voir `docs/adr/0004-pas-de-ci.md`.
- Avant chaque commit/push : `cargo test --workspace` et `pnpm build` en local doivent passer.
- Ne jamais committer de clé (signature updater : `*.key`, `.env` sont ignorés).
- Ne jamais écraser un fichier utilisateur (utiliser `convix_core::output::resolve`).
- Nom définitif : **Convix** (anciennement le nom de code ConvertIO). Le garder centralisé (`src-tauri/tauri.conf.json` → `productName`, `package.json`, `README.md`, titre dans `src/App.tsx`).
- Tenir `docs/ROADMAP.md` à jour (✅/🔜/⬜) et noter toute nouvelle décision dans `docs/CADRAGE.md` ou un nouvel ADR.

## Carte du code
- `crates/convix-core` : formats, détection, `OperationDescriptor`/`OptionSpec`, trait `Converter`, `Registry`, sortie, annulation. **Toute la logique va dans les crates**, pas dans `src-tauri`.
- `crates/convix-{image,audio,pdf}` : un convertisseur chacun (PDF : lopdf + moteur PDF de Windows pour le rendu). Opération `multi_input` = un seul job avec tous les fichiers (fusion, images → PDF). Ajouter une conversion = ajouter un descripteur dans `operations_for` + le cas dans `run`. L'interface s'adapte seule.
- `crates/convix-modules` : modules externes (ffmpeg) téléchargés à la demande. Liste + URL + SHA-256 dans `crates/convix-modules/manifest.json` (nouvelle version = recalculer l'empreinte). Une opération qui en dépend déclare `requires: ["ffmpeg"]` : l'UI propose le téléchargement toute seule.
- `src-tauri/src` : `lib.rs` (démarrage, état), `commands.rs` (API pour l'UI), `cli.rs` (`--op <id> fichiers…`), `shell_integration.rs` (clic droit).
- `src/` : React. Appels Rust **uniquement** via `src/lib/api.ts`. Types miroirs dans `src/lib/types.ts` (à garder synchronisés avec Rust). Textes **uniquement** via `t("clé")` avec la clé dans `src/i18n/fr.json` **et** `en.json`.
- Les ids d'opération (`image.convert.png`) sont **stables** : presets et clic droit en dépendent. Ne pas les renommer.
- Un convertisseur n'est enregistré dans `build_registry` (`src-tauri/src/lib.rs`) **que lorsqu'il fonctionne** : l'interface ne doit jamais proposer une action qui renvoie « pas encore implémenté ».
- `src-tauri/src/jobs.rs` : exécution en lot (rayon), `catch_unwind` par fichier, réservation du nom de sortie (`output::reserve`) puis écriture atomique (`output::write_atomic`).
- Le menu clic droit liste `EXTENSIONS` dans `shell_integration.rs` : garder `src-tauri/windows/hooks.nsh` (désinstallation) synchronisé.

## Tester l'interface sans Tauri
`pnpm dev` puis ouvrir http://localhost:1420 dans un navigateur : `src/dev/mockTauri.ts` simule la partie Rust (dev uniquement). Dans la console : `__mockDrop()` simule un glisser-déposer. Config prête dans `.claude/launch.json` (`ui-mock`).

## Tester la vraie chaîne sans glisser-déposer
`pnpm tauri build --debug --no-bundle` puis `target\debug\convix.exe --op image.convert.png "C:\...\photo.webp"` : c'est exactement ce que fait le clic droit.

## Tester avec le vrai FFmpeg
`CONVIX_FFMPEG=<chemin de ffmpeg.exe> cargo test -p convix-audio` (conversions réelles vers chaque format) et `CONVIX_FFMPEG_7Z=<archive>.7z cargo test -p convix-modules` (extraction). Sans ces variables, ces tests sont sautés. Vrai téléchargement : `cargo test -p convix-modules -- --ignored`.

## Commandes
- `pnpm app:dev` — app en développement
- `cargo test --workspace` — tests moteur
- `pnpm build` — vérif TypeScript + build interface
- `pnpm app:build` — installeur NSIS
- `pnpm release -Version X.Y.Z [-Publish]` — release signée + GitHub Release (voir `docs/RELEASE.md`). La clé `%USERPROFILE%.taurinvix.key` ne doit jamais entrer dans le dépôt. La version vient de `Cargo.toml` (`[workspace.package]`), pas de `tauri.conf.json`.
- Journal de l'app : `%LOCALAPPDATA%io.github.luz0gg.convixogsnvix.log` (premier endroit où regarder en cas de bug).
