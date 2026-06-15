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
cargo xtask release-check v0.1.0-preview.1
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
./scripts/release/update-changelog.ps1 -Tag v0.1.2-preview.1 -Repository KingYoSun/kukuri -PreviousTag v0.1.1-preview.1
```

The `changelog` job needs `contents: write` and `pull-requests: write` permissions (already set in the workflow) to push the branch and open the PR.

## Third-party Notices

Before publishing a preview release, generate and review the Rust and desktop npm dependency
license inventories from the release tag.

The distribution notice lives at `docs/THIRD_PARTY_NOTICES.md` and is included in the draft release
assets as `THIRD_PARTY_NOTICES.md`. Update the generator if a dependency requires specific
attribution text beyond the package-level license inventory.

```powershell
./scripts/release/generate-third-party-notices.ps1
./scripts/release/generate-third-party-notices.ps1 -Check
```

## Manual Smoke

Use the exact draft release assets:

1. Install on a clean Windows 10 user profile.
2. Install on a clean Windows 11 user profile.
3. Confirm launch, Community Node `ready`, starter topic, public post, reply/thread, private channel, DM when a test peer is available, local notification inbox, and diagnostic report export.
4. Install a previous preview and update to the draft version.
5. Confirm identity, local DB, Iroh data, Community Node config, private channel capability, and notification inbox state remain.
6. If unsigned, confirm the release notes explain SmartScreen warnings.

## Diagnostics And Feedback

Users should open `Settings -> Release`, copy or export the diagnostic report, and attach it to the preview feedback issue template. The default report omits secret keys, auth tokens, private channel secrets, invite/share tokens, DM bodies, and local DB paths.

## Data Safety

Updates must preserve identity, local DB state, Iroh data, Community Node settings, private channel
capability, and the local notification inbox. Uninstall, reset, and migration-failure guidance must
tell users to keep the app data directory when they need to retain state, and failures should show
actionable diagnostics instead of silently clearing data.
