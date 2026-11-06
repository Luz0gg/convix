# ADR 0004 — Pas de CI GitHub Actions pour l'instant

- **Statut :** accepté (2026-11-06)

## Décision
Aucun workflow GitHub Actions (ni tests, ni build, ni release automatisés) afin de ne pas consommer de crédits. Tests, builds et publication des releases se font en local (`cargo test`, `pnpm build`, `pnpm app:build`, `gh release create`).

## Conséquences
- Ne jamais créer de dossier `.github/workflows/` sans accord explicite.
- Avant chaque push : `cargo test --workspace` et `pnpm build` en local.
