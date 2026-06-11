# 2026-06-11 Release Readiness Execution Plan

## Summary

- この計画は `builder preview` を「配布できる」「更新できる」「問題報告を回収できる」状態にするための release readiness plan です。
- 既存の local notification inbox は product activity の durable local inbox として維持します。
- OS 通知は既存 notification inbox とは別の feature slice として実装します。既存 `NotificationRow` / `NotificationView` は canonical activity inbox、OS 通知は opt-in delivery surface として扱います。
- updater / update notification は release infrastructure の一部として最優先で実装します。OS 通知とは別に、アプリ内 update status / banner / settings surface を持ちます。
- 初回 preview の配布対象は Windows installer、Linux は source-run fallback のままにします。

## Current Snapshot

- Windows release workflow は存在し、tag / manual dispatch で verify 後に Windows package を作り GitHub Release asset として publish します。
- Windows bundle target は NSIS installer です。
- `tauri-plugin-updater`、update manifest、update signature artifact、update UI はまだ release surface に含まれていません。
- local notification inbox v1 は mention / reply / repost / quote_repost / direct_message / followed の activity inbox として存在します。
- OS toast / push / dismiss / archive は local notification inbox v1 の scope 外です。
- Community Node diagnostics と troubleshooting docs はありますが、ユーザーが GitHub feedback に貼れる redacted diagnostics export はまだ明示的な release gate ではありません。
- production CSP、Windows code signing、updater signing、installer trust story は release gate として固定する必要があります。

## Release Readiness Principles

1. **更新可能性を先に固める**
   - preview 配布後の hotfix を成立させるため、installer 公開より先に updater path を完成させる。
   - manual download だけに依存しない。

2. **通知 surface を分離する**
   - product activity は existing local notification inbox に残す。
   - OS 通知は user opt-in、foreground/background policy、quiet mode、permission failure fallback を持つ別 layer にする。
   - update notification は release/update status として扱い、activity inbox の自動既読とは独立させる。

3. **ユーザーが問題を報告できる状態を exit criteria に含める**
   - connectivity が安定しても、preview では user environment 起因の failure が残る。
   - redacted diagnostics export を first preview の必須要件にする。

4. **identity / local data を壊さない**
   - updater / reinstall / migration / keyring fallback は preview の信頼性に直結する。
   - release validation は fresh install だけでなく old version update を必ず含める。

5. **security posture を release config として固定する**
   - dev convenience を release に持ち込まない。
   - CSP、deep-link validation、signed updater artifact、signed Windows installer を release gate に含める。

## Milestone Exit Criteria

A build is release-ready when all of the following are true:

- [ ] Windows installer can be downloaded from GitHub Releases and installed on a fresh Windows user profile.
- [ ] The installed app can check for updates, show update availability, install the update, relaunch, and preserve local data.
- [ ] Update artifacts are signed and verified by the app before install.
- [ ] Windows installer / executable are code-signed or the release explicitly documents unsigned-preview risk and mitigation.
- [ ] Release workflow publishes installer artifacts, updater artifacts, signatures, checksums, and release notes consistently.
- [ ] A user can export or copy redacted diagnostics for feedback.
- [ ] Community Node failure states remain readable and recoverable from settings.
- [ ] Existing local notification inbox still works after update.
- [ ] OS notification implementation, if shipped in the preview, is opt-in and independent from local notification inbox storage.
- [ ] Privacy / data storage / feedback data copy is visible from README or in-app About / Settings.
- [ ] CSP and release security settings are production-safe.

## Workstreams

| Priority | Workstream | Status | Output | Notes |
| --- | --- | --- | --- | --- |
| P0 | Version / channel discipline | planned | single release version source, preview channel convention | Use `vX.Y.Z-preview.N`; keep `tauri.conf.json`, Tauri crate, desktop package version synchronized. |
| P0 | Updater foundation | planned | Tauri updater plugin, updater config, signing key process | Add Rust/JS plugin deps, `check -> download -> install -> relaunch` runtime surface. |
| P0 | Update notification UI | planned | app-internal update banner + settings/about status | Do not store update notices in existing activity notification inbox by default. |
| P0 | Release workflow updater artifacts | planned | updater bundle, `.sig`, manifest, checksums | Extend `.github/workflows/kukuri-release.yml`. |
| P0 | Windows code signing | planned | signed EXE/MSI, CI secret handling | If certificate is unavailable for first preview, document risk and add manual validation gate. |
| P0 | Install / update E2E | planned | old -> new update scenario | Validate identity, DB, community-node config, notification inbox, private channel state. |
| P0 | Diagnostics export | planned | redacted report copy/export action | Include app version, OS, sync status, Community Node state, last errors, config shape, no secrets. |
| P0 | Production security config | planned | CSP, release-only capability review, deep-link validation audit | Current release config must not rely on `csp: null`. |
| P1 | OS notification slice | planned | opt-in OS notification delivery | Separate from `notifications` table; consume events/status but own permission/settings state. |
| P1 | Data safety / reset / backup UX | planned | backup/export/reset docs and maybe settings actions | Clarify identity loss, keyring fallback, local DB location, reinstall behavior. |
| P1 | Privacy and data storage copy | planned | README / runbook / app settings copy | State what is local, what is sent to Community Node, what diagnostics includes. |
| P1 | First-run release onboarding | planned | in-app checklist / happy path hints | Guide user through ready -> starter topic -> post/reply -> private channel -> feedback. |
| P1 | DB migration safety | planned | migration smoke, update backup policy | Add old-version DB fixture and migration failure UX. |
| P1 | Third-party notices | planned | generated OSS notices | Bundle or link from About / release notes. |
| P1 | Feedback home | planned | GitHub issue/discussion template and deep link | Pre-fill diagnostics checklist and expected feedback categories. |
| P2 | Staged rollout / rollback | deferred | dynamic update server or channel split | Static `latest-preview.json` is enough for first preview. |
| P2 | Crash reporting / telemetry | deferred | opt-in crash report | Do not add network telemetry before privacy copy and consent model are ready. |
| P2 | Accessibility release pass | planned | keyboard/screen-reader checklist | Include nav rail, dialogs, notification list, settings drawer, update prompt. |

## Execution Plan

### Phase 0: Release Baseline Audit

Goal: freeze the baseline and make release work visible.

Tasks:

- [ ] Decide first preview tag format, e.g. `v0.1.0-preview.1`.
- [ ] Add a release checklist issue or tracking board.
- [ ] Confirm target OS matrix: Windows 10 / Windows 11 for packaged preview, Linux source-run only.
- [ ] Confirm release branch policy: direct tag from `main` or release branch.
- [ ] Add a version sync check to `cargo xtask doctor` or a dedicated release check.
- [ ] Add `docs/runbooks/release.md` after the workflow is finalized.

Acceptance:

- One canonical release checklist exists.
- Version/channel convention is documented.
- Release candidate build can be produced from a reproducible command.

### Phase 1: Updater Foundation

Goal: installed preview builds can update without manual reinstall.

Tasks:

- [ ] Add `tauri-plugin-updater` to `apps/desktop/src-tauri/Cargo.toml`.
- [ ] Add `@tauri-apps/plugin-updater` to `apps/desktop/package.json`.
- [ ] Register the updater plugin in Tauri startup.
- [ ] Configure updater public key and endpoints in `tauri.conf.json` / release override config.
- [ ] Enable updater artifact creation for Windows bundles.
- [ ] Define update manifest naming:
  - `latest-preview.json` for preview channel.
  - `latest.json` reserved for stable channel.
- [ ] Add update status types to frontend API layer:
  - `idle`
  - `checking`
  - `up_to_date`
  - `available`
  - `downloading`
  - `ready_to_restart`
  - `failed`
- [ ] Add manual "Check for updates" action in Settings / About.
- [ ] Add non-blocking update banner when update is available.
- [ ] Add restart prompt after install is ready.
- [ ] Add error copy that distinguishes network failure, manifest unavailable, signature failure, and install failure.

Acceptance:

- A locally installed old build can discover a newer build from a test manifest.
- Signature mismatch refuses installation.
- Offline check fails gracefully.
- Update status is independent from local activity notification inbox.

### Phase 2: Release Workflow and Signing

Goal: GitHub Releases produce all artifacts required by installer users and updater users.

Tasks:

- [ ] Extend `.github/workflows/kukuri-release.yml` to publish:
  - NSIS installer.
  - updater bundle artifacts.
  - `.sig` files.
  - `latest-preview.json`.
  - checksums.
- [ ] Store updater private key in GitHub Actions secrets.
- [ ] Keep updater public key in repository config.
- [ ] Add release workflow validation that fails if manifest points to missing assets.
- [ ] Add Windows code-signing step when certificate is available.
- [ ] If code signing is not ready for first preview, add explicit release note copy and manual SmartScreen validation.
- [ ] Add artifact retention and artifact names that distinguish preview / stable.

Acceptance:

- Release workflow output is enough for both fresh install and update install.
- Generated manifest references immutable release asset URLs.
- Manual workflow dispatch can create a draft release without publishing it immediately.

### Phase 3: Update E2E and Data Safety

Goal: updates preserve user data and failures do not strand users.

Tasks:

- [ ] Build `v0.1.0-preview.1` and `v0.1.0-preview.2` test artifacts.
- [ ] Create a manual or automated update scenario:
  - install old build,
  - create identity,
  - wait for Community Node ready,
  - add starter topic,
  - post/reply,
  - create or join private channel,
  - receive or create local notification,
  - update to new build,
  - verify all state is present.
- [ ] Add migration fixture for at least one old DB.
- [ ] Document reinstall behavior: data preserved vs removed.
- [ ] Document keyring fallback behavior and file fallback risk.
- [ ] Add user-visible startup error for migration/open DB failure.

Acceptance:

- Update preserves identity, DB, Iroh data, community-node config, private-channel capabilities, and notification inbox.
- Failed update can be retried.
- Migration failure produces actionable diagnostics rather than a blank app.

### Phase 4: Diagnostics and Feedback Loop

Goal: preview users can provide actionable feedback without leaking secrets.

Tasks:

- [ ] Add diagnostics export/copy action in Settings.
- [ ] Include:
  - app version,
  - release channel,
  - OS and architecture,
  - sync status,
  - discovery mode,
  - Community Node session phase / last error / retry-after,
  - active path and peer counts,
  - subscribed topic count,
  - notification unread count,
  - recent non-secret error messages,
  - update status and last update error.
- [ ] Exclude or redact:
  - secret keys,
  - auth tokens,
  - private-channel capability secrets,
  - invite/share tokens,
  - raw DM content,
  - raw local DB paths if they include sensitive usernames unless user chooses full report.
- [ ] Add GitHub feedback URL or template copy.
- [ ] Update `docs/runbooks/mvp-user-quickstart.md` to ask users to attach diagnostics.
- [ ] Update `docs/runbooks/mvp-troubleshooting.md` with update and diagnostics sections.

Acceptance:

- A non-technical user can produce a useful bug report in one minute.
- Diagnostics output is safe to paste publicly by default.

### Phase 5: Production Security Hardening

Goal: preview builds have a clear, bounded security posture.

Tasks:

- [ ] Replace release `csp: null` with a production CSP.
- [ ] Review Tauri capabilities and ensure only required permissions are enabled.
- [ ] Audit deep-link parsing and reject unsupported schemes / malformed tokens.
- [ ] Ensure update endpoints are HTTPS.
- [ ] Ensure updater signature verification is required for all update installs.
- [ ] Add third-party notice generation.
- [ ] Add privacy/data storage copy to README and/or app settings.

Acceptance:

- Release config is intentionally stricter than dev config.
- Deep-link handling cannot silently import malformed or unintended data.
- Users can understand where data is stored and what leaves the device.

### Phase 6: OS Notification Slice

Goal: OS notifications are available without changing the semantics of local notification inbox.

Scope:

- This slice is separate from `NotificationRow` persistence.
- Existing local notification inbox remains the durable source for product activity history.
- OS notification delivery is a best-effort surface for user attention.

Tasks:

- [ ] Add OS notification plugin/dependency for desktop.
- [ ] Add notification settings:
  - master enable/disable,
  - direct messages,
  - mentions/replies,
  - follows/reposts if desired,
  - quiet mode / do not disturb if simple enough,
  - preview text on/off.
- [ ] Add permission request flow.
- [ ] Add delivery policy:
  - do not notify for self-authored events,
  - do not notify when the relevant pane is focused unless setting allows,
  - do not include private DM text if preview text is disabled,
  - dedupe by notification id or source event id.
- [ ] Decide whether OS notification click opens:
  - DM pane for direct message,
  - thread/topic for reply/mention/repost,
  - author pane for followed.
- [ ] Add tests for settings and dedupe policy.
- [ ] Add manual QA for permission denied / foreground / background / app closed where supported.

Acceptance:

- Turning OS notifications off does not affect local notification inbox.
- Clearing or reading local inbox does not implicitly change OS notification permission settings.
- OS notification click navigation matches existing notification click navigation where possible.

### Phase 7: Final Release Candidate Pass

Goal: one tagged candidate can be declared preview-ready.

Tasks:

- [ ] Run release workflow in draft mode.
- [ ] Install draft artifact on clean Windows 10 and Windows 11 machines.
- [ ] Complete happy path:
  - launch,
  - Community Node ready,
  - starter topic,
  - public post,
  - reply/thread,
  - private channel,
  - DM if test peer is available,
  - notification inbox,
  - diagnostics export.
- [ ] Complete update path from previous RC.
- [ ] Verify release notes include:
  - preview scope,
  - known limitations,
  - update behavior,
  - data storage/privacy note,
  - feedback instructions,
  - troubleshooting link.
- [ ] Publish release after final smoke.

Acceptance:

- Draft release can be promoted without changing artifacts.
- Known limitations are documented before users download the installer.

## Validation Matrix

| Path | Gate |
| --- | --- |
| workspace static check | `cargo xtask check` |
| Rust tests | `cargo xtask rust-test` |
| desktop lint/typecheck | `cargo xtask desktop-lint` |
| desktop unit tests | `cargo xtask desktop-test` |
| Storybook build | `cargo xtask desktop-storybook` |
| browser UI tests | `cargo xtask desktop-browser-test` |
| Tauri compile check | `cargo xtask tauri-check` |
| Windows package | `cargo xtask desktop-package` on Windows |
| smoke scenario | `cargo xtask e2e-smoke` |
| Community Node connectivity | `cargo xtask scenario community_node_public_connectivity` |
| updater test | install old build -> update to new build -> verify data |
| diagnostics test | export redacted report and inspect for secrets |
| OS notification test | permission allow/deny, foreground/background, click routing |

## Release Checklist

- [ ] `README.ja.md` and `README.md` describe preview scope and Windows installer path.
- [ ] `docs/runbooks/mvp-user-quickstart.md` includes update check and diagnostics feedback steps.
- [ ] `docs/runbooks/mvp-troubleshooting.md` includes updater and install/update failure states.
- [ ] `docs/runbooks/release.md` exists and matches the workflow.
- [ ] Release workflow can create draft release from tag.
- [ ] Draft release contains installer, updater artifacts, signatures, checksums, manifest, and release notes.
- [ ] Installer is signed or unsigned-preview risk is explicitly documented.
- [ ] Updater manifest points to release assets for the same version/channel.
- [ ] Update signature verification is tested with valid and invalid signatures.
- [ ] Fresh install happy path is manually confirmed.
- [ ] Update happy path is manually confirmed.
- [ ] Reinstall behavior is manually confirmed.
- [ ] Diagnostics export is manually confirmed and redaction reviewed.
- [ ] Existing local notification inbox still passes activity notification scenarios.
- [ ] OS notification settings do not mutate local notification inbox behavior.
- [ ] Privacy/data storage copy is visible before or during first use.
- [ ] Known limitations are listed in release notes.

## Open Questions

- Should first preview require Windows code signing, or can the first internal/builder preview ship unsigned with explicit warning?
- Should updater manifests be hosted as GitHub Release assets only, or also mirrored to a stable project-owned URL?
- Should `latest-preview.json` be generated by CI or checked into a release metadata branch?
- Should diagnostics export be clipboard-only for first preview, or also write a ZIP/text file?
- Should OS notifications ship in the first public preview, or land immediately after updater and diagnostics?
- Should update checks run automatically on startup, on interval, or only manually for first preview?

## Non-goals For This Milestone

- General-public launch.
- macOS packaging/notarization.
- Linux binary packaging.
- Dynamic staged rollout server, unless static GitHub manifest proves insufficient.
- Cross-device notification sync.
- Push notification service.
- Mandatory telemetry.
- Full moderation tooling.
