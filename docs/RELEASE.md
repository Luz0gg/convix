# Publier une version de Convix

Tout se fait **depuis ton PC** : il n'y a pas de CI GitHub (voir [ADR 0004](adr/0004-pas-de-ci.md)).

## Comment marchent les mises à jour

1. Une version publiée est une **Release GitHub** `vX.Y.Z` qui contient trois fichiers : l'installeur `Convix_X.Y.Z_x64-setup.exe`, sa signature `.sig` et `latest.json`.
2. Au démarrage (réglage « Rechercher les mises à jour », activé par défaut), Convix lit `https://github.com/Luz0gg/convix/releases/latest/download/latest.json`.
3. Si la version y est plus récente, un bandeau propose « Installer et redémarrer ».
4. L'installeur téléchargé n'est lancé **que si sa signature correspond** à la clé publique écrite dans `src-tauri/tauri.conf.json` (`plugins.updater.pubkey`). Personne d'autre que toi ne peut donc pousser une mise à jour, même avec un accès au dépôt.

## La clé de signature — À SAUVEGARDER

- Clé privée : `%USERPROFILE%\.tauri\convix.key` (créée le 2026-11-06, sans mot de passe). **Jamais dans le dépôt** (`*.key` est ignoré).
- Clé publique : `%USERPROFILE%\.tauri\convix.key.pub`, recopiée dans `tauri.conf.json`.
- ⚠️ **Si la clé privée est perdue, les copies déjà installées ne pourront plus jamais se mettre à jour toutes seules.** Il faudrait alors réinstaller Convix à la main sur chaque PC. Copie `convix.key` sur une clé USB ou dans un gestionnaire de mots de passe.
- Si elle fuit (quelqu'un d'autre la possède) : générer une nouvelle paire (`pnpm tauri signer generate -w %USERPROFILE%\.tauri\convix.key`), mettre la nouvelle clé publique dans `tauri.conf.json`, publier une version. Les utilisateurs devront installer cette version-là à la main.

## Publier

Prérequis : `gh` connecté (`gh auth status`), branche `main` à jour, tout commité.

1. Essai à blanc (construit et signe, ne publie rien ; les fichiers du dépôt sont remis comme avant à la fin) :
   ```
   pnpm release -Version 1.0.0
   ```
2. Tester l'installeur produit dans `target\release\bundle\nsis\`.
3. Publier :
   ```
   pnpm release -Version 1.0.0 -Publish -Notes "Nouveautés : …"
   ```

Le script fait, dans l'ordre :
1. Il met la version dans `Cargo.toml` (source unique pour l'app) et dans `package.json`.
2. Il lance les tests (`cargo test --workspace`).
3. Il construit l'installeur signé (`pnpm tauri build`, qui exécute aussi `pnpm build`).
4. Il écrit `latest.json`.
5. Avec `-Publish` seulement : il fait un commit « Release vX.Y.Z », le pousse, puis crée la Release GitHub avec les 3 fichiers (c'est elle qui crée le tag : si elle échoue, rien n'est tagué et on relance simplement la même commande).

Une version avec un suffixe (`1.1.0-beta.1`) est publiée en **pré-version** : elle n'est jamais proposée en mise à jour automatique.

## Numéros de version

- `1.0.1` : corrections.
- `1.1.0` : nouvelles fonctions.
- `2.0.0` : changement qui casse quelque chose (réglages, ids d'opérations…).

## Avertissement Windows à l'installation

Sans certificat payant (décision du 2026-11-06), Windows SmartScreen affiche « Windows a protégé votre ordinateur » au premier lancement de l'installeur. Il faut cliquer sur « Informations complémentaires », puis « Exécuter quand même ». Les **mises à jour automatiques**, elles, passent sans cet écran.
