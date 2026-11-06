# ADR 0003 — Clic droit Explorateur via le registre utilisateur (HKCU)

- **Statut :** accepté (2026-11-06)

## Contexte
Le chef de projet veut convertir depuis l'Explorateur sans ouvrir l'app, dès la V1.

## Décision
- Menu en cascade classique sous `HKCU\Software\Classes\SystemFileAssociations\.<ext>\shell\ConvertIO`, sans droits admin, généré depuis le registre des opérations.
- Commande : `ConvertIO.exe --op <operation-id> "<fichier>"`, regroupée par le plugin `single-instance` quand plusieurs fichiers sont sélectionnés.
- Interrupteur dans les réglages + nettoyage à la désinstallation.

## Conséquences
- Sous Windows 11, l'entrée apparaît dans « Afficher plus d'options » (Maj+F10). L'intégration au nouveau menu Windows 11 (COM `IExplorerCommand` + package MSIX sparse, qui exige une signature de code) est repoussée après la V1.
