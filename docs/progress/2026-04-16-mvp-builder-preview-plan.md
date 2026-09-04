# 2026-04-16 MVP Builder Preview Plan

## Summary

- このマイルストーンは general launch ではなく `builder preview` の切り出しです。
- capability baseline は [2026-03-10-foundation.md](./2026-03-10-foundation.md) を維持し、その上に `初回体験 / 配布 / 説明 / feedback loop` を載せます。
- Issue #885を現行マイルストーンへ組み込み、Windows NSISに加えてLinux x86_64 AppImageとx86_64／aarch64利用者向けCLIをPreview Releaseへ載せます。実装順と境界は#886〜#890、データ分類はADR 0049を正本とします。
- Community Node は最後まで単一の概念として扱います。配布候補と利用者追加 Node はどちらも、Node 固有文書への明示同意後にセッションを自動確立・維持します。
- current preview surface は `launch -> default product Columns -> profile setup -> community node consent -> node ready -> starter topic -> post/reply -> private channel -> feedback` です。

## Current Snapshot

- runtime は `community-node.json` を後方互換で読みつつ、fresh install でだけ Tauri 配布設定の Community Node 一覧を preload します。preview 配布設定は `https://api.kukuri.app` を候補として指定しますが、汎用 runtime はこの domain を知りません。保存済み一覧（空を含む）が常に優先されます。
- Node 同意前は UI 操作起点の公開 manifest / 法務文書取得以外の通信を開始しません。利用者が提示された現行文書へ明示同意すると、`authenticate -> server consent sync -> metadata refresh` を自動で進めます。
- token は期限 5 分前から proactive refresh し、`401` は `re-authenticate -> retry`、`403 CONSENT_REQUIRED` はローカル同意が server の現行 required 文書をカバーする場合だけ `server consent sync -> retry` します。版が上がった場合は自動受諾せず再同意を求めます。
- desktop settings は textarea editor をやめ、Community Node の単一 list 上で `base URL`, diagnostics, consent / troubleshooting actions を扱います。
- starter topic は `kukuri:topic:general`, `kukuri:topic:dev`, `kukuri:topic:test` を default とします（#805 で `demo / iroh / nostr / operators` から変更）。
- 保存済みlayoutがないfresh installでは、`demo Timeline -> 自分のProfile -> Explore -> Notifications -> Messages`のpin済みColumnを表示し、Timelineをactiveにします。既存layoutは補完・移行しません。

## Preview Surface

- packaged distribution: Windows NSIS installer via GitHub Releases
- 配布予定: Linux x86_64 AppImage、x86_64／aarch64 `kukuri-cli`をGitHub Preview Releaseで配布（#885）
- source実行による代替導線: Linux（#885完了までは現行導線として維持）
- docs: root README, `docs/runbooks/mvp-user-quickstart.md`, `docs/runbooks/mvp-troubleshooting.md`
- feedback home: GitHub を canonical とし、preview announcement 前に Discussions か同等の feedback surface を有効化する

## Workstreams

| Workstream | Status | Type | Notes |
| --- | --- | --- | --- |
| Community Node consent/session boundary | landed | repo change | Node 別明示同意を接続前提とし、同意後の session 維持を自動化 |
| Distribution Community Node config | landed | repo change | preview 候補を Tauri 配布設定へ隔離し、削除・置換後に復活しない契約を追加 |
| Startup session maintenance | landed | repo change | 明示同意済み Node の auth / server consent sync / metadata refresh を自動化 |
| Token expiry auto re-auth | landed | repo change | proactive refresh, `401` retry, `403` conditional accept を追加 |
| Community Node unified settings surface | landed | repo change | official/custom split を作らず row editor に置換 |
| Starter topics default | landed | repo change | desktop shell default tracked topics を 4 件に変更 |
| Default product Column layout | landed | repo change | fresh installだけ主要5 Columnを表示し、既存layoutを維持 |
| Preview docs refresh | landed | repo change | README, docs index, user quickstart, troubleshooting を追加 |
| Device backup / restore | landed | repo change | 使用中の1アカウントを1暗号化ファイルへ保存し、staging検証・rollback・再同意付きで復元 |
| Windows release workflow | landed | repo change | tag / manual dispatch で NSIS asset を Release に載せる |
| Linux共通基盤／常駐プロセス／CLI通信規約 | planned | repo change | #886〜#888。GUIと分離したprofileを単一の常駐プロセスが所有し、版管理されたローカルprotocolで非OS固有backendとの同等性を提供する |
| Linux AppImage／リリース統合 | planned | repo change | #889〜#890。Ubuntu 22.04でのbuild、Ubuntu 22.04／Debian 12での実環境smoke test、複数platform成果物の全件成功を要求するリリースを追加する |
| Seed content on hosted preview node | planned | launch op | project-owned author で preview topics を事前投入する |
| GitHub feedback surface | planned | launch op | Discussions category か同等の GitHub feedback home を整備する |

## Validation Matrix

| Path | Gate |
| --- | --- |
| runtime compile | `cargo check -p kukuri-desktop-runtime` |
| frontend settings regression | `npx pnpm@10.16.1 --dir apps/desktop test` |
| workspace fast path | `cargo xtask check` |
| UI path | `cargo xtask desktop-ui-check` |
| community-node path | `cargo xtask cn-test` |
| smoke path | `cargo xtask e2e-smoke` |
| preview scenario | `cargo xtask scenario community_node_public_connectivity` |

## Launch Checklist

- [x] GitHub Release で Windows installer を公開（`v0.1.x-preview.*` を GitHub Releases に公開済み）
- [x] README 冒頭を builder preview 導線へ更新（root `README.md` に「Builder Preview」節あり）
- [x] `mvp-user-quickstart` と `mvp-troubleshooting` を公開（`docs/runbooks/` に存在）
- [ ] hosted preview node 上で starter topic seed content を確認
- [ ] GitHub feedback surface を preview copy から辿れるようにする
- [ ] packaged Windows app で `launch -> ready -> post -> reply -> private channel` を手動確認
- [ ] #885の全親AC／INVARを満たし、Linux AppImageとx86_64／aarch64 CLIをWindows成果物と同じPreview Releaseへ完全に公開

## Assumptions

- 配布候補と利用者追加 Node は同じ同意・セッション契約を使い、Node の種別差を作りません。
- community-node server endpoint contract 自体は変更しません。
- Linuxのバイナリ配布と利用者向けCLIは#885としてこのマイルストーンの完了条件に含めます。hosted Storybook、一般公開、モデレーションツールは引き続き含めません。
