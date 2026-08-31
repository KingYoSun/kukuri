# Release Runbook

## Scope

- Initial preview channel: `preview`.
- Tag format: `vX.Y.Z-preview.N`, for example `v0.1.0-preview.1`.
- Windows is the only packaged preview target.
- Linux remains source-run only.
- The release workflow extends `cargo xtask desktop-package`; it does not use `tauri-action` as the primary build path.

## Required Secrets

- `TAURI_SIGNING_PRIVATE_KEY`: updater signing private key.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: optional updater signing key password.
- `TAURI_UPDATER_PUBLIC_KEY`: updater public key that matches `TAURI_SIGNING_PRIVATE_KEY`; the workflow patches this into `tauri.conf.json` before the Windows package build.

Generate a keypair outside the repository:

```bash
cd apps/desktop
npx pnpm@10.16.1 tauri signer generate --write-keys <secure-private-key-path>
```

Store the private key contents in `TAURI_SIGNING_PRIVATE_KEY` and the `.pub` contents in `TAURI_UPDATER_PUBLIC_KEY`.

Windows code signing certificates are optional for the first preview. If code signing is not configured, the release notes must state that the preview is unsigned and that SmartScreen warnings are expected.

## Local Gates

```bash
cargo xtask release-check v0.1.8-preview.1
cargo xtask check
cargo xtask test
cargo xtask e2e-smoke
```

On a Windows host:

```powershell
cargo xtask desktop-package
.\scripts\release\test-create-preview-assets.ps1
```

## Workflow

1. Create a tag matching `vX.Y.Z-preview.N`.
2. Run `Kukuri Release` with `workflow_dispatch` and the tag, or push the tag.
3. The workflow runs:
   - `validate-release-inputs`
   - `linux-verify`
   - `windows-package`
   - `changelog`
   - `release-assets`
   - `publish-draft`
4. `publish-draft` creates a GitHub draft release by default.
5. Smoke the draft release assets without replacing them.
6. Publish the draft from GitHub Releases after Windows 10 / Windows 11 smoke passes.

## Release Assets

The draft release must include:

- Windows NSIS installer.
- Tauri updater bundle.
- `.sig` file.
- `latest-preview.json`.
- `SHA256SUMS.txt`.
- `release-assets.txt`.
- `manual-smoke-checklist.md`.
- `RELEASE_NOTES_DRAFT.md`.

`latest-preview.json` must embed the `.sig` file contents in `platforms.windows-x86_64.signature`. It must not point `signature` at a `.sig` URL.

`latest-preview.json` は **UTF-8 without BOM** でなければならない。Windows PowerShell 5.1 の
`Set-Content -Encoding UTF8` はBOMを付けるため、manifestの手動生成・上書きには使わない。
必ず `scripts/release/create-preview-assets.ps1` を使う。このscriptは
`System.Text.UTF8Encoding(false)` で書き出し、`test-create-preview-assets.ps1` はbyte-levelでBOMを拒否する。

公開前の最低検査:

```powershell
./scripts/release/test-create-preview-assets.ps1

$manifestPath = '<release-assets>/latest-preview.json'
$bytes = [IO.File]::ReadAllBytes($manifestPath)
if ($bytes.Length -ge 3 -and
    $bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF) {
  throw 'latest-preview.json has a UTF-8 BOM'
}
$manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
if (-not $manifest.platforms.'windows-x86_64'.signature) {
  throw 'embedded updater signature is missing'
}
```

`RELEASE_NOTES_DRAFT.md` embeds the changelog section for the release tag (the `## Changes` block with per-pull-request links) ahead of the static `Included` / `Known limits` / `Feedback` content. The `changelog` job produces that section; see [Changelog](#changelog).

## Changelog

The repository keeps a [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)-style `CHANGELOG.md` at the root. Per-release sections are generated automatically.

- The `changelog` job (`.github/workflows/kukuri-release.yml`) runs after `linux-verify` and `windows-package` succeed. It checks out the default branch with full history (`fetch-depth: 0`).
- `scripts/release/update-changelog.ps1 -Tag <tag> -Repository <owner/repo>` walks `git log <previous-tag>..<tag>`, classifies each non-merge commit by Conventional Commit type into Features / Fixes / Other, links every `(#NNN)` reference to its pull request, and inserts a `## [<tag>] - <date>` section below `## [Unreleased]`. Re-running for the same tag replaces the section rather than duplicating it.
- The job uploads the tag's section as the `kukuri-changelog-section` artifact, which `release-assets` embeds into `RELEASE_NOTES_DRAFT.md`. This upload happens before the CHANGELOG commit, so the release notes never depend on the pull request step.
- The default branch requires pull requests, so the job does not push to it directly. It commits the updated `CHANGELOG.md` to a `chore/changelog-<tag>` branch and opens (or reuses) a pull request titled `docs: update CHANGELOG for <tag>`. Merge that PR to record the release in `CHANGELOG.md`. This step is best-effort (`continue-on-error`): if it fails, the release still publishes and the CHANGELOG can be updated manually.
- Changes released in `v0.1.1-preview.1` and earlier are not backfilled; they remain in GitHub Releases. Automated entries start from the next preview.

To preview the generated section locally before tagging (no commit, branch, or PR is created):

```powershell
./scripts/release/update-changelog.ps1 -Tag v0.1.8-preview.1 -Repository KingYoSun/kukuri -PreviousTag v0.1.7-preview.1
```

The `changelog` job needs `contents: write` and `pull-requests: write` permissions (already set in the workflow) to push the branch and open the PR.

## Third-party Notices

Before publishing a preview release, validate the non-code asset rights manifest, then generate
and review the Rust, desktop npm, and bundled asset license inventories from the release tag.

The distribution notice lives at `docs/THIRD_PARTY_NOTICES.md` and is included in the draft release
assets as `THIRD_PARTY_NOTICES.md`. Exact non-code asset paths, SHA-256 digests, provenance,
source-only versus bundled-binary scope, modification status, redistribution conditions, and credit
requirements live in `docs/ASSET_MANIFEST.json`. The repository root MIT license does not replace a
third-party asset license.

```powershell
cargo xtask asset-check
./scripts/release/generate-third-party-notices.ps1
./scripts/release/generate-third-party-notices.ps1 -Check
```

For every non-code asset addition, update, or deletion under `apps/desktop/public/`,
`apps/desktop/app-icon.png`, or `apps/desktop/src-tauri/icons/`:

1. Update the manifest entry and exact file SHA-256. Do not reuse the previous digest after editing.
2. Record the author and rights holder, source and acquisition or creation date, license text or URL,
   modification status, commercial use, repository and binary redistribution, and credit condition.
3. For generated or generation-assisted material, also record the service, model, input rights, and
   output terms URL. Do not mark unknown provenance or redistribution as allowed.
4. Run the three commands above and review the generated asset sections separately from the package
   inventories.
5. Confirm the draft release contains the generated `THIRD_PARTY_NOTICES.md` before publication.

`cargo xtask asset-check` and `cargo xtask release-check` fail on unregistered, missing, duplicate,
or changed files and on unknown or insufficient rights. Pull request CI runs the same asset check.

## Manual Smoke

Use the exact draft release assets:

1. Install on a clean Windows 10 user profile.
2. Install on a clean Windows 11 user profile.
3. Confirm launch, Community Node `ready`, starter topic, public post, reply/thread, private channel, DM when a test peer is available, local notification inbox, and diagnostic report export.
4. Install a previous preview and update to the draft version.
5. Confirm identity, local DB, Iroh data, Community Node config, private channel capability, and notification inbox state remain.
6. If unsigned, confirm the release notes explain SmartScreen warnings.

### 公開後のupdater smoke

GitHub UI上のasset確認だけでは不十分。旧previewが実際に読む短いURLをCDN経由で取得し、
redirect先の一時的な `release-assets.githubusercontent.com` URLをrunbookや設定へ保存しない。

```powershell
$out = Join-Path ([IO.Path]::GetTempPath()) 'kukuri-latest-preview.json'
$url = 'https://github.com/KingYoSun/kukuri/releases/latest/download/latest-preview.json'
curl.exe -L --fail --silent --show-error "$url`?verify=$([DateTimeOffset]::UtcNow.ToUnixTimeSeconds())" -o $out

$bytes = [IO.File]::ReadAllBytes($out)
if ($bytes.Length -ge 3 -and
    $bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF) {
  throw 'published latest-preview.json has a UTF-8 BOM'
}
$manifest = Get-Content -LiteralPath $out -Raw -Encoding UTF8 | ConvertFrom-Json
$manifest.version
$manifest.platforms.'windows-x86_64'.url
if (-not $manifest.platforms.'windows-x86_64'.signature) {
  throw 'published updater signature is missing'
}
```

続けて次を確認する。

1. manifestのversionがrelease予定versionと一致する。
2. installer URLがHTTP成功し、release assetのファイル名と一致する。
3. embedded signatureが同releaseの `.sig` 内容と一致する。
4. `SHA256SUMS.txt` を再取得し、公開assetを再計算して全件一致する。
5. 旧previewから「更新を確認」→download→installを実行し、clean installとは別に成功を確認する。

公開済み bundle が設定済み public key で成功し、同じ bundle を 1 byte 改変すると必ず拒否される
ことは次の自動 smoke で確認する。manifest の embedded signature と配布 bundle を取得するため、
network access が必要。

```powershell
./scripts/release/test-published-updater-signature.ps1 -Tag v0.1.8-preview.1
```

assetを差し替えた場合は `SHA256SUMS.txt` も必ず更新する。GitHub/CDNの切替後に上記の短いURLを
cache-busting query付きで再取得し、旧previewを再起動するか更新確認を再実行する。

### workflow後段だけが失敗した場合

OIDC / artifact attestationなど、署名済みpackageのbuild/upload後の後段だけが外部障害で失敗した場合は、
失敗runの**同一artifact**を取得して次の全条件を満たす場合だけ手動公開へ切り替えられる。

- Windows package buildとTauri updater署名が成功済み
- `create-preview-assets.ps1` でassetを再生成
- release asset smoke、BOM検査、全checksum一致
- installerのversionとtag/source SHA一致
- release notesに欠落したattestationと理由を明記

build・署名自体が失敗したrunのartifactを公開してはならない。手動回復時もinstallerや `.sig` を
作り直して混在させず、同一run由来の一式として扱う。

## Diagnostics And Feedback

Users should open `Settings -> Release`, copy or export the diagnostic report, and attach it to the preview feedback issue template. The default report omits secret keys, auth tokens, private channel secrets, invite/share tokens, DM bodies, and local DB paths.

## Data Safety

Updates must preserve identity, local DB state, Iroh data, Community Node settings, private channel
capability, and the local notification inbox. Uninstall, reset, and migration-failure guidance must
tell users to keep the app data directory when they need to retain state, and failures should show
actionable diagnostics instead of silently clearing data.
