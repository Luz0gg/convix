# Convix release, built and published from this PC (no CI: see docs/adr/0004-pas-de-ci.md).
#
#   pnpm release -Version 1.0.0                 # dry run: builds, signs, prepares latest.json
#   pnpm release -Version 1.0.0 -Publish        # + commit, push, GitHub Release (tag created there)
#
# Needs: the signing key in %USERPROFILE%\.tauri\convix.key (never in the repo), `gh` logged in.
# Full guide: docs/RELEASE.md
param(
    [Parameter(Mandatory = $true)][string]$Version,
    [string]$Notes = "",
    [switch]$Publish
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

function Step($text) { Write-Host "`n==> $text" -ForegroundColor Cyan }
function Fail($text) { Write-Host "ERREUR : $text" -ForegroundColor Red; exit 1 }
# Native commands (git, gh, cargo) don't stop the script by themselves in PowerShell 5.1.
function Run([string]$what, [scriptblock]$command) {
    & $command
    if ($LASTEXITCODE -ne 0) { Fail $what }
}

if ($Version -notmatch '^\d+\.\d+\.\d+(-[0-9A-Za-z.]+)?$') { Fail "version invalide « $Version » (attendu : 1.2.3)" }
$prerelease = $Version.Contains("-") # 1.1.0-beta.1: published as a pre-release, never offered as an update
$keyPath = Join-Path $env:USERPROFILE ".tauri\convix.key"
if (-not (Test-Path $keyPath)) { Fail "clé de signature introuvable : $keyPath (voir docs/RELEASE.md)" }
if ($Publish) {
    if (git status --porcelain) { Fail "des modifications ne sont pas commitées" }
    if ((git rev-parse --abbrev-ref HEAD) -ne "main") { Fail "il faut être sur la branche main" }
    git fetch --quiet --tags origin
    if (git tag --list "v$Version") { Fail "la version v$Version existe déjà" }
    $behind = git rev-list --count "HEAD..origin/main"
    if ($behind -ne "0") { Fail "main n'est pas à jour avec GitHub (git pull)" }
    gh auth status 2>$null | Out-Null
    if ($LASTEXITCODE -ne 0) { Fail "gh n'est pas connecté (gh auth login)" }
}

Step "Version $Version"
$utf8 = New-Object System.Text.UTF8Encoding $false
$versionFiles = @("Cargo.toml", "package.json", "Cargo.lock")
$backup = @{}
foreach ($f in $versionFiles) { $backup[$f] = [IO.File]::ReadAllBytes("$root\$f") }
# The dry run must leave the repository as it was (otherwise -Publish refuses to run afterwards).
function Restore-Versions { foreach ($f in $versionFiles) { [IO.File]::WriteAllBytes("$root\$f", $backup[$f]) } }

$cargo = [IO.File]::ReadAllText("$root\Cargo.toml")
$cargo = [regex]::Replace($cargo, '(?m)(\[workspace\.package\][^\[]*?^version\s*=\s*)"[^"]*"', "`${1}""$Version""")
[IO.File]::WriteAllText("$root\Cargo.toml", $cargo, $utf8)
$pkg = [IO.File]::ReadAllText("$root\package.json")
$pkg = [regex]::Replace($pkg, '(?m)^(\s*"version":\s*)"[^"]*"', "`${1}""$Version""", 1)
[IO.File]::WriteAllText("$root\package.json", $pkg, $utf8)

try {
    Step "Tests du moteur"
    cargo test --workspace
    if ($LASTEXITCODE -ne 0) { throw "les tests ne passent pas" }

    Step "Installeur signé"
    $env:TAURI_SIGNING_PRIVATE_KEY = [IO.File]::ReadAllText($keyPath).Trim()
    $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ""
    pnpm tauri build
    if ($LASTEXITCODE -ne 0) { throw "la construction a échoué" }
}
catch {
    Restore-Versions
    Fail $_
}
finally {
    Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY -ErrorAction SilentlyContinue
}

$bundle = "$root\target\release\bundle\nsis"
$installer = Get-ChildItem $bundle -Filter "Convix_$($Version)_*-setup.exe" | Select-Object -First 1
if (-not $installer) { Restore-Versions; Fail "installeur introuvable dans $bundle" }
$sigFile = "$($installer.FullName).sig"
if (-not (Test-Path $sigFile)) { Restore-Versions; Fail "signature introuvable : $sigFile" }

Step "latest.json (lu par les copies installées pour se mettre à jour)"
$url = "https://github.com/Luz0gg/convix/releases/download/v$Version/$($installer.Name)"
if (-not $Notes) { $Notes = "Convix $Version" }
$latest = [ordered]@{
    version   = $Version
    notes     = $Notes
    pub_date  = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    platforms = [ordered]@{
        "windows-x86_64" = [ordered]@{ signature = [IO.File]::ReadAllText($sigFile).Trim(); url = $url }
    }
}
$latestPath = "$bundle\latest.json"
[IO.File]::WriteAllText($latestPath, ($latest | ConvertTo-Json -Depth 5), $utf8)

$size = [math]::Round($installer.Length / 1MB, 1)
Write-Host "`nPrêt : $($installer.Name) ($size Mo), .sig et latest.json dans $bundle" -ForegroundColor Green

if (-not $Publish) {
    Restore-Versions
    Write-Host "Essai à blanc : rien n'a été publié, les fichiers du dépôt sont revenus à leur état d'origine." -ForegroundColor Yellow
    Write-Host "Relancer avec -Publish pour publier."
    exit 0
}

Step "Commit et push"
git add Cargo.toml Cargo.lock package.json
git diff --cached --quiet
if ($LASTEXITCODE -ne 0) { Run "le commit a échoué" { git commit -m "Release v$Version" } }
Run "le push a échoué" { git push origin main }

Step "Release GitHub (crée aussi le tag v$Version)"
$commit = (git rev-parse HEAD).Trim()
$ghArgs = @("release", "create", "v$Version", $installer.FullName, $sigFile, $latestPath, "--target", $commit, "--title", "Convix $Version", "--notes", $Notes)
if ($prerelease) { $ghArgs += "--prerelease" }
Run "la création de la release a échoué (rien n'est tagué : relancer la même commande)" { gh @ghArgs }
git fetch --quiet --tags origin
Write-Host "`nPublié : https://github.com/Luz0gg/convix/releases/tag/v$Version" -ForegroundColor Green
