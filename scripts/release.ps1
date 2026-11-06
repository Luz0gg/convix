# Convix release, built and published from this PC (no CI: see docs/adr/0004-pas-de-ci.md).
#
#   pnpm release -Version 1.0.0                 # dry run: builds, signs, prepares latest.json
#   pnpm release -Version 1.0.0 -Publish        # + commit, tag, push, GitHub Release
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

if ($Version -notmatch '^\d+\.\d+\.\d+(-[0-9A-Za-z.]+)?$') { Fail "version invalide « $Version » (attendu : 1.2.3)" }
$keyPath = Join-Path $env:USERPROFILE ".tauri\convix.key"
if (-not (Test-Path $keyPath)) { Fail "clé de signature introuvable : $keyPath (voir docs/RELEASE.md)" }
if ($Publish) {
    if (git status --porcelain) { Fail "des modifications ne sont pas commitées" }
    if ((git rev-parse --abbrev-ref HEAD) -ne "main") { Fail "il faut être sur la branche main" }
    if (git tag --list "v$Version") { Fail "la version v$Version existe déjà" }
    gh auth status 2>$null | Out-Null
    if ($LASTEXITCODE -ne 0) { Fail "gh n'est pas connecté (gh auth login)" }
}

Step "Version $Version"
$utf8 = New-Object System.Text.UTF8Encoding $false
$cargo = [IO.File]::ReadAllText("$root\Cargo.toml")
$cargo = [regex]::Replace($cargo, '(?m)(\[workspace\.package\][^\[]*?^version\s*=\s*)"[^"]*"', "`${1}""$Version""")
[IO.File]::WriteAllText("$root\Cargo.toml", $cargo, $utf8)
$pkg = [IO.File]::ReadAllText("$root\package.json")
$pkg = [regex]::Replace($pkg, '(?m)^(\s*"version":\s*)"[^"]*"', "`${1}""$Version""", 1)
[IO.File]::WriteAllText("$root\package.json", $pkg, $utf8)

Step "Tests du moteur"
cargo test --workspace
if ($LASTEXITCODE -ne 0) { Fail "les tests ne passent pas" }

Step "Installeur signé"
$env:TAURI_SIGNING_PRIVATE_KEY = [IO.File]::ReadAllText($keyPath).Trim()
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ""
pnpm tauri build
if ($LASTEXITCODE -ne 0) { Fail "la construction a échoué" }

$bundle = "$root\target\release\bundle\nsis"
$installer = Get-ChildItem $bundle -Filter "Convix_$($Version)_*-setup.exe" | Select-Object -First 1
if (-not $installer) { Fail "installeur introuvable dans $bundle" }
$sigFile = "$($installer.FullName).sig"
if (-not (Test-Path $sigFile)) { Fail "signature introuvable : $sigFile" }

Step "latest.json (lu par les copies installées pour se mettre à jour)"
$url = "https://github.com/Luz0gg/convix/releases/download/v$Version/$($installer.Name)"
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
    Write-Host "Essai à blanc : rien n'a été publié. Relancer avec -Publish pour publier." -ForegroundColor Yellow
    Write-Host "(Cargo.toml et package.json portent maintenant la version $Version.)"
    exit 0
}

Step "Commit, tag et push"
git add Cargo.toml Cargo.lock package.json
git commit -m "Release v$Version"
git tag "v$Version"
git push origin main "v$Version"
if ($LASTEXITCODE -ne 0) { Fail "le push a échoué" }

Step "Release GitHub"
if (-not $Notes) { $Notes = "Convix $Version" }
gh release create "v$Version" $installer.FullName $sigFile $latestPath --title "Convix $Version" --notes $Notes
if ($LASTEXITCODE -ne 0) { Fail "la création de la release a échoué" }
Write-Host "`nPublié : https://github.com/Luz0gg/convix/releases/tag/v$Version" -ForegroundColor Green
