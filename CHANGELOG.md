# Changelog

All notable changes to kukuri are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Releases use the preview tag scheme `vX.Y.Z-preview.N`.

Per-release sections under this header are generated automatically by the
`changelog` job in `.github/workflows/kukuri-release.yml`, which runs
`scripts/release/update-changelog.ps1` against the git history between the
previous release tag and the release tag, links each entry to its pull
request, and commits the result. See `docs/runbooks/release.md` for the
release workflow.

Changes released in `v0.1.1-preview.1` and earlier are tracked in the
[GitHub Releases](https://github.com/KingYoSun/kukuri/releases) instead of this
file; automated changelog entries start from the next preview release.

## [Unreleased]

## [v0.1.7-preview.1] - 2026-08-26

### Features

- show key decks on fresh start ([#785](https://github.com/KingYoSun/kukuri/pull/785))

### Fixes

- harden multilingual UI layouts ([#783](https://github.com/KingYoSun/kukuri/pull/783))

### Other

- bump preview version to 0.1.7 ([#786](https://github.com/KingYoSun/kukuri/pull/786))
- make README a user and contributor entry point ([#784](https://github.com/KingYoSun/kukuri/pull/784))
- update CHANGELOG for v0.1.6-preview.1 ([#781](https://github.com/KingYoSun/kukuri/pull/781))

## [v0.1.6-preview.1] - 2026-08-25

### Features

- 配布素材の権利と再配布条件を管理する ([#778](https://github.com/KingYoSun/kukuri/pull/778))
- 投稿コンテンツの権利表明と限定的利用許諾を追加 ([#777](https://github.com/KingYoSun/kukuri/pull/777))
- add scoped case retention and legal holds ([#776](https://github.com/KingYoSun/kukuri/pull/776))
- add rights infringement request intake ([#760](https://github.com/KingYoSun/kukuri/pull/760), [#775](https://github.com/KingYoSun/kukuri/pull/775))
- add signed withdrawals and transmission prevention ([#774](https://github.com/KingYoSun/kukuri/pull/774))
- add staged column workspace features ([#772](https://github.com/KingYoSun/kukuri/pull/772))
- add Issue #748 Wave 6 mobile lifecycle ([#758](https://github.com/KingYoSun/kukuri/pull/758))
- add Issue #748 Wave 5 variable spans ([#757](https://github.com/KingYoSun/kukuri/pull/757))
- add Issue #748 Wave 4 Control Center ([#756](https://github.com/KingYoSun/kukuri/pull/756))
- 管理画面の確認・結果ページを dashboard と共通 shell にする ([#740](https://github.com/KingYoSun/kukuri/pull/740), [#747](https://github.com/KingYoSun/kukuri/pull/747))
- 管理画面の操作を標準で有効化 ([#742](https://github.com/KingYoSun/kukuri/pull/742))

### Fixes

- pin release XML assets to LF ([#780](https://github.com/KingYoSun/kukuri/pull/780))
- Issue #765 Column 振る舞いの残課題を解消 ([#771](https://github.com/KingYoSun/kukuri/pull/771))
- Issue #748 監査 blocker B1-B5 を修正 ([#769](https://github.com/KingYoSun/kukuri/pull/769))
- Community Node provider alertの誤検知を分離 ([#746](https://github.com/KingYoSun/kukuri/pull/746))
- BlobText投稿本文をCommunity Indexへ投影する ([#745](https://github.com/KingYoSun/kukuri/pull/745))
- preserve native select contrast on Linux ([#743](https://github.com/KingYoSun/kukuri/pull/743))
- 接続設定の横スクロールを防ぐ ([#744](https://github.com/KingYoSun/kukuri/pull/744))

### Other

- bump preview version to 0.1.6 ([#779](https://github.com/KingYoSun/kukuri/pull/779))
- remove legacy shell projections ([#773](https://github.com/KingYoSun/kukuri/pull/773))
- Issue #768 Validation マトリクスと review record を補完 ([#770](https://github.com/KingYoSun/kukuri/pull/770))
- [codex][refactor:delete] 旧ShellFrame経路を削除 ([#759](https://github.com/KingYoSun/kukuri/pull/759))
- Issue #748 Wave 3: scope Columns and composers ([#755](https://github.com/KingYoSun/kukuri/pull/755))
- Issue #748 Wave 2: migrate production surfaces to Columns ([#754](https://github.com/KingYoSun/kukuri/pull/754))
- extract existing surface renderers ([#753](https://github.com/KingYoSun/kukuri/pull/753))
- Issue #748 Wave 1: use icon tabs in the Column header ([#752](https://github.com/KingYoSun/kukuri/pull/752))
- align detail visual baselines with Linux CI ([#751](https://github.com/KingYoSun/kukuri/pull/751))
- Issue #748 Wave 1: Column Canvas foundation ([#750](https://github.com/KingYoSun/kukuri/pull/750))
- Issue #748 Wave 0: 可変span Column Canvasのreview prototype ([#749](https://github.com/KingYoSun/kukuri/pull/749))
- update CHANGELOG for v0.1.5-preview.1 ([#737](https://github.com/KingYoSun/kukuri/pull/737))

## [v0.1.5-preview.1] - 2026-08-21

### Features

- 安定エラーコードを通信境界で定数化し契約試験と画面判別を揃える ([#712](https://github.com/KingYoSun/kukuri/pull/712), [#730](https://github.com/KingYoSun/kukuri/pull/730))
- 動画添付を media 対象として個別に通報できるようにする ([#697](https://github.com/KingYoSun/kukuri/pull/697), [#724](https://github.com/KingYoSun/kukuri/pull/724))
- add Community Node index client ([#671](https://github.com/KingYoSun/kukuri/pull/671))
- add audited Community Node admin actions ([#660](https://github.com/KingYoSun/kukuri/pull/660))

### Fixes

- propagate CN distance policy to Terraform ([#736](https://github.com/KingYoSun/kukuri/pull/736))
- 訂正版再発行後に利用者が審査結果を確認できるようにする ([#710](https://github.com/KingYoSun/kukuri/pull/710), [#732](https://github.com/KingYoSun/kukuri/pull/732))
- 索引申請の受付で索引参照の構成と有効化を確認する ([#713](https://github.com/KingYoSun/kukuri/pull/713), [#731](https://github.com/KingYoSun/kukuri/pull/731))
- 非公開チャンネルの索引参照を所属者に限定する ([#711](https://github.com/KingYoSun/kukuri/pull/711), [#728](https://github.com/KingYoSun/kukuri/pull/728))
- 異議申し立て審査の確認画面に変更前後の値を表示する ([#701](https://github.com/KingYoSun/kukuri/pull/701), [#727](https://github.com/KingYoSun/kukuri/pull/727))
- 異議申し立て審査の入力値を保存前に検証する ([#700](https://github.com/KingYoSun/kukuri/pull/700), [#726](https://github.com/KingYoSun/kukuri/pull/726))
- 信頼・関係応答の対象識別子を要求対象へ照合する ([#723](https://github.com/KingYoSun/kukuri/pull/723))
- 通報送信時に受付先を構成済みノードと同一オリジンへ限定し転送を追跡しない ([#722](https://github.com/KingYoSun/kukuri/pull/722))
- 通報先候補で提供中能力と責任範囲を厳密に照合する ([#721](https://github.com/KingYoSun/kukuri/pull/721))
- 信頼・関係機能を同意済みで提供中のノードに限定する ([#705](https://github.com/KingYoSun/kukuri/pull/705), [#720](https://github.com/KingYoSun/kukuri/pull/720))
- 索引ノードの利用可否変更時に古い選択・結果・索引申請を失効させる ([#719](https://github.com/KingYoSun/kukuri/pull/719))
- 参加拒否後に保持したトークンで自己修復経路が再認証を繰り返す問題を止める ([#718](https://github.com/KingYoSun/kukuri/pull/718))
- 添付対象のリスク判定を画面から異議申し立てできるようにする ([#717](https://github.com/KingYoSun/kukuri/pull/717))
- 異議申し立ての発行元識別子を公開ノード情報の node_id と一致させる ([#716](https://github.com/KingYoSun/kukuri/pull/716))
- 通報画面で開いた時に取得した最新ノード情報だけを送信先候補にする ([#714](https://github.com/KingYoSun/kukuri/pull/714))
- 観測記録の90日保持を読み取り時にも強制 ([#695](https://github.com/KingYoSun/kukuri/pull/695))
- 信頼・関係画面の取得文脈を固定 ([#694](https://github.com/KingYoSun/kukuri/pull/694))
- 索引結果の通報文脈を固定 ([#693](https://github.com/KingYoSun/kukuri/pull/693))
- 異議申し立ての対象と解消済み状態を修正 ([#689](https://github.com/KingYoSun/kukuri/pull/689))
- 添付の観測元と通報経路を修正 ([#688](https://github.com/KingYoSun/kukuri/pull/688))
- 索引申請の秘密値確認を送信ごとに消費 ([#687](https://github.com/KingYoSun/kukuri/pull/687))

### Other

- bump preview version to 0.1.5 ([#735](https://github.com/KingYoSun/kukuri/pull/735))
- stabilize docs sync relay tests ([#734](https://github.com/KingYoSun/kukuri/pull/734))
- 運営者審査の有効化設定を compose と terraform に配線する ([#709](https://github.com/KingYoSun/kukuri/pull/709), [#729](https://github.com/KingYoSun/kukuri/pull/729))
- クライアント視点の結合試験に異議申し立ての一続きと距離利用停止の結線確認を加える ([#725](https://github.com/KingYoSun/kukuri/pull/725))
- 通報画面試験で送信ボタンの出現を待ち合わせる ([#715](https://github.com/KingYoSun/kukuri/pull/715))
- 招待制参加認証の運用手順を更新 ([#686](https://github.com/KingYoSun/kukuri/pull/686))
- Issue #680 の異議申し立て審査を追加 ([#682](https://github.com/KingYoSun/kukuri/pull/682))
- 課題 #669 リスク判定への異議申し立てを追加 ([#681](https://github.com/KingYoSun/kukuri/pull/681))
- 非公開チャンネルのランデブー鍵を世代秘密から派生 ([#679](https://github.com/KingYoSun/kukuri/pull/679))
- Community Node の招待認証にクライアントを対応させる ([#678](https://github.com/KingYoSun/kukuri/pull/678))
- 課題 #666 の観測元記録と分散通報を実装 ([#677](https://github.com/KingYoSun/kukuri/pull/677))
- Add trust relation desktop UI and harness ([#676](https://github.com/KingYoSun/kukuri/pull/676))
- Issue #665: trust/relation desktop clientを追加 ([#675](https://github.com/KingYoSun/kukuri/pull/675))
- Fix distance opt-out E2E contract ([#674](https://github.com/KingYoSun/kukuri/pull/674))
- Implement community node distance opt-out ([#673](https://github.com/KingYoSun/kukuri/pull/673))
- Implement Community Node indexing request flows ([#672](https://github.com/KingYoSun/kukuri/pull/672))
- run workflows with Rust 1.92 ([#662](https://github.com/KingYoSun/kukuri/pull/662))
- update Rust and TypeScript dependencies ([#661](https://github.com/KingYoSun/kukuri/pull/661))
- update CHANGELOG for v0.1.4-preview.1 ([#659](https://github.com/KingYoSun/kukuri/pull/659))

## [v0.1.4-preview.1] - 2026-08-11

### Features

- automate remaining preview operations ([#656](https://github.com/KingYoSun/kukuri/pull/656))

### Fixes

- do not treat clean classifier as detection ([#654](https://github.com/KingYoSun/kukuri/pull/654))
- share active peers with media fetcher ([#653](https://github.com/KingYoSun/kukuri/pull/653))
- report deployed indexer stack ([#652](https://github.com/KingYoSun/kukuri/pull/652))
- publish data retention disclosure ([#650](https://github.com/KingYoSun/kukuri/pull/650))
- honor readiness activation after startup ([#649](https://github.com/KingYoSun/kukuri/pull/649))
- mount readiness operator config ([#648](https://github.com/KingYoSun/kukuri/pull/648))
- sync live peers and serve disclosures ([#647](https://github.com/KingYoSun/kukuri/pull/647))
- emit updater manifest without BOM ([#646](https://github.com/KingYoSun/kukuri/pull/646))

### Other

- bump preview version to 0.1.4 ([#658](https://github.com/KingYoSun/kukuri/pull/658))
- update CHANGELOG for v0.1.3-preview.1 ([#657](https://github.com/KingYoSun/kukuri/pull/657))
- add community node production rollout runbook ([#655](https://github.com/KingYoSun/kukuri/pull/655))
- cover replica query failure end to end ([#651](https://github.com/KingYoSun/kukuri/pull/651))
- update CHANGELOG for v0.1.3-preview.1 ([#645](https://github.com/KingYoSun/kukuri/pull/645))

## [v0.1.3-preview.1] - 2026-08-07

### Features

- MediaFetcher の本番実装(blob の一時 fetch で media scan を実働化)
- media 参照ごとの safety scan + derived タグの index 相乗り
- appeal 経路 + operator レビュー
- openai-compatible-vlm の provider 解決 + operator config/readiness
- operator policy 注入 + suspected visibility override + derived_tags
- OpenAI-compatible VLM moderation provider crate
- suspected_threshold(70)/signal visibility policy + derived tags foundation
- report ConnectionPath::RelayFallback from connection diagnostics
- community node trust / relation foundation (CommunityLocalTrust read surface) ([#415](https://github.com/KingYoSun/kukuri/pull/415), [#427](https://github.com/KingYoSun/kukuri/pull/427))
- Project Arachnid Shield known-CSAM provider 統合 ([#391](https://github.com/KingYoSun/kukuri/pull/391), [#426](https://github.com/KingYoSun/kukuri/pull/426))
- fail-closed community indexing 本体と search/discovery/recommendation 除外 ([#404](https://github.com/KingYoSun/kukuri/pull/404), [#425](https://github.com/KingYoSun/kukuri/pull/425))
- add model C index ingestion ([#423](https://github.com/KingYoSun/kukuri/pull/423))
- persist signed moderation events ([#407](https://github.com/KingYoSun/kukuri/pull/407))
- add uuid event id generator ([#403](https://github.com/KingYoSun/kukuri/pull/403))
- add system scan clock ([#402](https://github.com/KingYoSun/kukuri/pull/402))
- add safety runtime adapter ([#400](https://github.com/KingYoSun/kukuri/pull/400))
- add safety readiness CLI ([#397](https://github.com/KingYoSun/kukuri/pull/397))
- add safety domain model ([#396](https://github.com/KingYoSun/kukuri/pull/396))
- generate low-cost tfvars from operator config ([#394](https://github.com/KingYoSun/kukuri/pull/394))
- add GCP community node Terraform ([#392](https://github.com/KingYoSun/kukuri/pull/392))
- add community node admission controls ([#390](https://github.com/KingYoSun/kukuri/pull/390))
- add community node consent review flow ([#389](https://github.com/KingYoSun/kukuri/pull/389))
- add app legal consent gate ([#388](https://github.com/KingYoSun/kukuri/pull/388))
- iroh 1.0 に更新 ([#385](https://github.com/KingYoSun/kukuri/pull/385))
- バックグラウンドOS通知とトレイ常駐を追加 ([#304](https://github.com/KingYoSun/kukuri/pull/304), [#378](https://github.com/KingYoSun/kukuri/pull/378))
- OS通知クリックで対象投稿を開く ([#377](https://github.com/KingYoSun/kukuri/pull/377))
- 更新DL/検証後に再起動確認プロンプトを追加 ([#319](https://github.com/KingYoSun/kukuri/pull/319), [#376](https://github.com/KingYoSun/kukuri/pull/376))
- capability 別リスクと推奨対応ガイドを生成 ([#359](https://github.com/KingYoSun/kukuri/pull/359), [#375](https://github.com/KingYoSun/kukuri/pull/375))
- コミュニティノード側の通報受信エンドポイントと運営者確認導線 ([#370](https://github.com/KingYoSun/kukuri/pull/370), [#371](https://github.com/KingYoSun/kukuri/pull/371))
- コミュニティノード宛ての分散通報ルーティング ([#310](https://github.com/KingYoSun/kukuri/pull/310), [#369](https://github.com/KingYoSun/kukuri/pull/369))
- community node 依存度 / capability scope 表示 ([#357](https://github.com/KingYoSun/kukuri/pull/357), [#368](https://github.com/KingYoSun/kukuri/pull/368))
- content provenance と responsible capability metadata を追加 ([#358](https://github.com/KingYoSun/kukuri/pull/358), [#367](https://github.com/KingYoSun/kukuri/pull/367))
- public manifest endpoint ([#356](https://github.com/KingYoSun/kukuri/pull/356), [#366](https://github.com/KingYoSun/kukuri/pull/366))
- manifest を型付き共有スキーマ化し authority scope/P2P境界を追加 ([#355](https://github.com/KingYoSun/kukuri/pull/355), [#365](https://github.com/KingYoSun/kukuri/pull/365))
- コミュニティノード運営者向け文書生成CLIを追加 ([#352](https://github.com/KingYoSun/kukuri/pull/352), [#364](https://github.com/KingYoSun/kukuri/pull/364))
- スレッドツリーの枝線・タイムラインのリプライ表示・ブックマークアイコン・メンションz-indexを改善 ([#351](https://github.com/KingYoSun/kukuri/pull/351))

### Fixes

- make private secret file persistence atomic via temp+fsync+rename
- refresh topic rendezvous presence independently of the bootstrap heartbeat
- decode NULL option columns as None instead of empty values
- log the discarded operation context on internal errors
- bind metaverse translations to shell locale ([#543](https://github.com/KingYoSun/kukuri/pull/543))
- localize metaverse room surfaces ([#542](https://github.com/KingYoSun/kukuri/pull/542))
- localize direct message errors ([#535](https://github.com/KingYoSun/kukuri/pull/535))
- harden GCP community node COS bootstrap ([#393](https://github.com/KingYoSun/kukuri/pull/393))

### Other

- bump preview version to 0.1.3 ([#644](https://github.com/KingYoSun/kukuri/pull/644))
- Complete community node operational hardening ([#643](https://github.com/KingYoSun/kukuri/pull/643))
- #617 の昇格・開示同期の記録を追加する
- 届出用構成図・役務説明・モデレーション方針を実態へ同期する (#617 T5+T6)
- 保存・保持の開示にデータ区分と保存先を追加する (#617 T4)
- 安全性走査プロバイダの外部送信を operator config から動的に開示する (#617 T3)
- 昇格した capability の説明を実装済みのデータフローへ更新する (#617 T2)
- community index / moderation / local trust を提供中へ昇格する (#617 T1)
- #616 の実機解禁記録と readiness 運用手順を追加する
- generate-tfvars が読み取り面の環境変数 gate を features から導出する
- 信頼・申し立て・関係・復旧の E2E を追加する (#616 T6)
- 全構成 E2E の土台と許可・不許可・障害経路を追加する (#616 T4+T5)
- readiness 全項目合格の記録を関門にして読み取り面を有効化する
- readinessの走査網羅と全構成の実行時判定を実測で確定させる（#616 T2）
- cn-cli readinessでプロバイダ疎通確認を実行しprovider_credential_validを確定させる（#616 T1）
- CIのlow-cost planへ実運用tfvarsをrepository variable経由で供給する
- ローカルinitで混入したlock file差分を戻す
- COSの/var noexecでbackup / cert-renew timerが実行できない問題を直す
- e2e smoke specのtopic selectorを表示名基準へ更新する
- topic IDの名前空間prefixをUIから隠す
- docs / example / test中の実運用由来のprivate subnetをRFC 5737のdocumentationレンジへ置換する
- xtaskのcn compose envへ#615で必須化した変数を追加しCIのcompose起動を直す
- GCP runbookへindex / moderation stackの配備・再構築・rollback手順を追記する（#615 T7）
- ArcadeDB CREATE PROPERTYのIF NOT EXISTS位置を文法どおりデータ型の前へ直す
- ローカルinitで混入したlock file差分を戻す
- GCP low-cost Terraformへcn-indexer・ArcadeDB・moderation secrets・relation timerを追加する（#615 T4-T6）
- ArcadeDB image既定値を26.8.1へpinする
- 標準composeへArcadeDB・cn-indexer・relation定期解析を追加する（#615 T2/T3）
- cn-operatorのdeploy configをindexer stack対応に拡張しtfvars生成へ配線する（#615 T1）
- cn-indexer imageをGHCR workflowに追加しpublish前のvalidate-config smokeを組み込む（#614 T3-T5）
- rustfmt差分を修正する
- cn-indexerのproduction featureをArachnid+VLMに分離しvalidate-configモードを追加する（#614 T1/T2）
- 実iroh 2台と実ArcadeDBの統合テストを追加し、#613の進捗を文書化する（#613 T4/T5）
- cn-indexerを常駐ワーカー化し観測状態を追加する（#613 T2/T3）
- cargo fmtの整形差分を修正する
- cn-indexerのingest経路を本番依存で結線する（#613 T1）
- Add UI review record and previews for developer mode
- Add developer mode toggle hiding WIP features and diagnostics
- rust-toolchainにrust-analyzerを追加
- cargo fmt(clippy --fix 後の let-chain 整形追随)
- ADR 0028 §7 実装追補 + runbook + progress doc + 実機 e2e テスト
- split connection path tests out of relay_connectivity.rs
- declare the new rendezvous scheduler test in the lock classification
- extract the shared hint roundtrip test helpers
- record the standing decisions for observer pull and fake transport scope
- gate unused shell CSS selectors with a vitest sweep
- extract the shared remote fetch loop into iroh-node
- share the CN response types instead of client mirrors
- fix references that rotted behind later refactors
- pin runtimeApi request literals to the generated DTO types
- generate the IPC request DTOs into types.generated.ts
- stop triggering kukuri-fast on docs-only changes
- ratchet baseline up for the published retry predicates
- publish the private channel import retry contract
- write placement conventions for refactored boundaries
- reconcile oversized baseline after B9 merges
- ratchet oversized baseline down to current line counts
- delete dead frontend exports and CSS selectors
- remove cn-core protocol shims and dead cn-protocol fns
- delete dead client-side pub APIs and unreachable branch
- extract remaining section view models
- apply rustfmt to lock_contract
- add lock classification contract for desktop-runtime
- unify hand-rolled stable polls onto poll_until
- make section loaders the single source of shell data fetching
- remove tauri-plugin-notification
- replace notification permission plugin invokes with app commands
- derive CN packages from cargo metadata
- split cn-cli command modules
- cover cn-cli timestamp helpers
- centralize CN tracing setup ([#567](https://github.com/KingYoSun/kukuri/pull/567))
- move safety composition to runtime ([#566](https://github.com/KingYoSun/kukuri/pull/566))
- pin safety persistence failures ([#565](https://github.com/KingYoSun/kukuri/pull/565))
- scenario wire orphaned scenarios into nightly
- refactor scope desktop test locks
- refactor add resource scoped test locks
- refactor centralize cn test env gates
- refactor split app api slow tests ([#560](https://github.com/KingYoSun/kukuri/pull/560))
- refactor shared test diagnostics ([#559](https://github.com/KingYoSun/kukuri/pull/559))
- refactor test support foundation ([#558](https://github.com/KingYoSun/kukuri/pull/558))
- type trust and relation errors ([#557](https://github.com/KingYoSun/kukuri/pull/557))
- type indexing errors ([#556](https://github.com/KingYoSun/kukuri/pull/556))
- type bootstrap and report errors ([#555](https://github.com/KingYoSun/kukuri/pull/555))
- type auth and consent errors ([#554](https://github.com/KingYoSun/kukuri/pull/554))
- freeze HTTP error contracts ([#553](https://github.com/KingYoSun/kukuri/pull/553))
- type private channel import errors ([#552](https://github.com/KingYoSun/kukuri/pull/552))
- characterize private channel import errors ([#551](https://github.com/KingYoSun/kukuri/pull/551))
- route helpers through service handles ([#550](https://github.com/KingYoSun/kukuri/pull/550))
- add service handles composition root ([#549](https://github.com/KingYoSun/kukuri/pull/549))
- make docs fetch policy explicit ([#548](https://github.com/KingYoSun/kukuri/pull/548))
- narrow internal core helpers ([#547](https://github.com/KingYoSun/kukuri/pull/547))
- make core exports explicit ([#546](https://github.com/KingYoSun/kukuri/pull/546))
- complete epoch handoff grant rename ([#545](https://github.com/KingYoSun/kukuri/pull/545))
- freeze epoch handoff legacy contract ([#544](https://github.com/KingYoSun/kukuri/pull/544))
- add metaverse shell actions ([#541](https://github.com/KingYoSun/kukuri/pull/541))
- extract metaverse room session ([#540](https://github.com/KingYoSun/kukuri/pull/540))
- extract metaverse room view ([#539](https://github.com/KingYoSun/kukuri/pull/539))
- extract metaverse room controls ([#538](https://github.com/KingYoSun/kukuri/pull/538))
- extract metaverse room discovery ([#537](https://github.com/KingYoSun/kukuri/pull/537))
- characterize metaverse room boundaries ([#536](https://github.com/KingYoSun/kukuri/pull/536))
- split shell presentation selectors ([#534](https://github.com/KingYoSun/kukuri/pull/534))
- centralize shell route state ([#533](https://github.com/KingYoSun/kukuri/pull/533))
- extract timeline view models ([#532](https://github.com/KingYoSun/kukuri/pull/532))
- split shell section loaders ([#531](https://github.com/KingYoSun/kukuri/pull/531))
- extract shell dialog controller ([#530](https://github.com/KingYoSun/kukuri/pull/530))
- unify shell focus scrolling ([#529](https://github.com/KingYoSun/kukuri/pull/529))
- extract shell share preview hook ([#528](https://github.com/KingYoSun/kukuri/pull/528))
- [fix] 同期ステータスを event push で即時反映 (WP-Q2b) ([#527](https://github.com/KingYoSun/kukuri/pull/527))
- [refactor] 通知ステータスを event push で即時反映 (WP-Q2 PR5) ([#526](https://github.com/KingYoSun/kukuri/pull/526))
- [refactor] CN セッション 7 マップを単一エントリに統合 (WP-Q2 PR4) ([#525](https://github.com/KingYoSun/kukuri/pull/525))
- [refactor] 起動エラー分類を文字列 contains から typed downcast へ (WP-Q2 PR3) ([#524](https://github.com/KingYoSun/kukuri/pull/524))
- [fix] get_community_node_statuses を読み取り専用化 (WP-Q2 PR2) ([#523](https://github.com/KingYoSun/kukuri/pull/523))
- [refactor] Reloadable* 3 ラッパーを declarative macro で生成 (WP-Q2 PR1) ([#522](https://github.com/KingYoSun/kukuri/pull/522))
- [refactor] main.tsx の browser mock seed を mocks/ へ移動 (WP-Q1 PR7) ([#521](https://github.com/KingYoSun/kukuri/pull/521))
- [refactor] 一回性の review Storybook 2 本を削除 (WP-Q1 PR6) ([#520](https://github.com/KingYoSun/kukuri/pull/520))
- [refactor] vestigial な endpoint_publish_task / dht_options 配管を除去 (WP-Q1 PR5b) ([#519](https://github.com/KingYoSun/kukuri/pull/519))
- [refactor] Rust dead: key_kind() と app-api の test 専用糖衣ラッパー 2 件を削除 (WP-Q1 PR5) ([#518](https://github.com/KingYoSun/kukuri/pull/518))
- [refactor] TS OS 通知 dead 5 関数と buildGameLink を削除 (WP-Q1 PR4) ([#517](https://github.com/KingYoSun/kukuri/pull/517))
- [refactor] dead な ShellTopBar 一式を削除 (WP-Q1 PR3) ([#516](https://github.com/KingYoSun/kukuri/pull/516))
- [refactor] Cargo 未使用依存 3 件の宣言削除 (WP-Q1 PR2) ([#515](https://github.com/KingYoSun/kukuri/pull/515))
- [refactor] .gitmodules 削除と .gitignore 旧レイアウト残骸の掃除 (WP-Q1 PR1) ([#514](https://github.com/KingYoSun/kukuri/pull/514))
- [refactor] shell-phase1.css を連続4分割 + スタイリング層ルール文書化 (WP-H8 PR4) ([#513](https://github.com/KingYoSun/kukuri/pull/513))
- [refactor] 重複 CSS 宣言を統合(冗長 99 規則削除)(WP-H8 PR3) ([#512](https://github.com/KingYoSun/kukuri/pull/512))
- [refactor] shell-phase1-legacy.css を shell-scoped-overrides.css へ改名 (WP-H8 PR2) ([#511](https://github.com/KingYoSun/kukuri/pull/511))
- [refactor] 未使用 CSS セレクタ 34 クラスを削除 (WP-H8 PR1) ([#510](https://github.com/KingYoSun/kukuri/pull/510))
- [refactor] community-node 型を ts-rs で生成物化・生成器を desktop-runtime へ移設 (WP-H7 PR3 / Stage 3b) ([#509](https://github.com/KingYoSun/kukuri/pull/509))
- [refactor] core/transport の残り IPC 型を ts-rs で生成物化 (WP-H7 PR3 / Stage 3a) ([#508](https://github.com/KingYoSun/kukuri/pull/508))
- [refactor] metaverse/game/live 型を ts-rs で生成物化 (WP-H7 PR3 / Stage 2) ([#507](https://github.com/KingYoSun/kukuri/pull/507))
- [refactor] IPC view 型を ts-rs で生成物化 (WP-H7 PR3 / Stage 1) ([#506](https://github.com/KingYoSun/kukuri/pull/506))
- [refactor] desktopApiMock をドメイン別ファイルへ分割 (WP-H7 PR2) ([#505](https://github.com/KingYoSun/kukuri/pull/505))
- [refactor] runtimeApi の mock 分岐をディスパッチヘルパへ集約 (WP-H7 PR1) ([#504](https://github.com/KingYoSun/kukuri/pull/504))
- [refactor] page/ コンポーネントの store 由来 props を子側購読へ (WP-H6 PR4) ([#503](https://github.com/KingYoSun/kukuri/pull/503))
- [refactor] DesktopShellState をドメインスライス合成へ分割 (WP-H6 PR3) ([#502](https://github.com/KingYoSun/kukuri/pull/502))
- [refactor] shell の全ストア購読を selector 購読へ移行 (WP-H6 PR2) ([#501](https://github.com/KingYoSun/kukuri/pull/501))
- [refactor] shell の Record 更新 / AsyncPanelState ヘルパー導入 (WP-H6 PR1) ([#500](https://github.com/KingYoSun/kukuri/pull/500))
- [refactor] 購読タスク管理を SubscriptionRegistry へ集約 (WP-H5 PR5) ([#499](https://github.com/KingYoSun/kukuri/pull/499))
- [refactor] rotate_private_channel をフェーズ分割 (WP-H5 PR4) ([#498](https://github.com/KingYoSun/kukuri/pull/498))
- [refactor] private channel import 3 系統をテンプレート統合 (WP-H5 PR3) ([#497](https://github.com/KingYoSun/kukuri/pull/497))
- [refactor] AuthorViewParts 抽出と timeline_runtime_support の二分割 (WP-H5 PR2) ([#496](https://github.com/KingYoSun/kukuri/pull/496))
- [refactor] service/mod.rs の glob 再輸出を明示 import へ (WP-H5 PR1) ([#495](https://github.com/KingYoSun/kukuri/pull/495))
- [refactor] cn-user-api lib.rs / contract.rs をドメイン別に分割 (WP-H4) ([#494](https://github.com/KingYoSun/kukuri/pull/494))
- [refactor:boundary] HTTP パス定数と request 型を cn-protocol で共有 (WP-H3 PR2) ([#493](https://github.com/KingYoSun/kukuri/pull/493))
- [refactor:boundary] cn-protocol 共有 wire crate を抽出 (WP-H3 PR1) ([#492](https://github.com/KingYoSun/kukuri/pull/492))
- [refactor:boundary] IrohDocsNode を kukuri-iroh-node crate へ移動 (WP-H2 PR2) ([#491](https://github.com/KingYoSun/kukuri/pull/491))
- [refactor] docs-sync / blob-service のピア管理を kukuri-transport へ共通化 (WP-H2 PR1) ([#490](https://github.com/KingYoSun/kukuri/pull/490))
- [refactor] ProjectionStore デフォルト実装のヘルパ関数化 (WP-H1 PR2) ([#489](https://github.com/KingYoSun/kukuri/pull/489))
- [refactor:boundary] ProjectionStore をドメイン別 sub-trait に分割 (WP-H1 PR1) ([#488](https://github.com/KingYoSun/kukuri/pull/488))
- [docs] 互換パス3本の撤去条件を明文化 (WP-C8) ([#487](https://github.com/KingYoSun/kukuri/pull/487))
- [fix] 判定根拠(basis)の導出を verdict 文脈対応にし ADR 0027 §2.2 の既知ギャップを解消 (WP-C7) ([#486](https://github.com/KingYoSun/kukuri/pull/486))
- [fix] CRLF checksum 自己修復を撤去し fail-loud 化 (WP-C6) ([#485](https://github.com/KingYoSun/kukuri/pull/485))
- [fix] endpoint secret を version + hex の自前形式で永続化 (WP-C5) ([#484](https://github.com/KingYoSun/kukuri/pull/484))
- [fix] GossipHint parse 失敗を warn ログとカウンタで可観測化 (WP-C4) ([#483](https://github.com/KingYoSun/kukuri/pull/483))
- [fix] IPC エラーを { code, message } 構造化封筒へ変更し文言非依存判定にする (WP-C3) ([#482](https://github.com/KingYoSun/kukuri/pull/482))
- [docs] WP-C2 完了報告を docs/progress に追加 ([#481](https://github.com/KingYoSun/kukuri/pull/481))
- [fix] #479 マージ時に復活した export ラッパーの persist 呼び出し残骸を除去 ([#480](https://github.com/KingYoSun/kukuri/pull/480))
- [refactor:boundary] capability 永続化を AppService の write-through callback へ集約 (WP-C2 T5) ([#479](https://github.com/KingYoSun/kukuri/pull/479))
- [contract] capability registry 永続 JSON の形状 fixture を追加 (WP-C2 T4) ([#478](https://github.com/KingYoSun/kukuri/pull/478))
- [fix] export 系ラッパーの capability persist 漏れを修正 (WP-C2 T1-T2) ([#476](https://github.com/KingYoSun/kukuri/pull/476))
- [fix] keyring set 失敗時に stale entry を削除して file fallback を有効化 (WP-C2 T3) ([#477](https://github.com/KingYoSun/kukuri/pull/477))
- [docs] WP-C1 完了報告を docs/progress に追加 (WP-C1 T5 記録) ([#475](https://github.com/KingYoSun/kukuri/pull/475))
- [fix] get_sync_status を読み取り専用化し CN セッション駆動をスケジューラへ一本化 (WP-C1 T4) ([#474](https://github.com/KingYoSun/kukuri/pull/474))
- [fix] CN セッション維持を desktop-runtime 内スケジューラで駆動 (WP-C1 T1-T3) ([#473](https://github.com/KingYoSun/kukuri/pull/473))
- [docs] 参照チェーン整合(README/AGENTS 順序 + progress 分離規約 + preview checklist)(WP-S8 T3) ([#472](https://github.com/KingYoSun/kukuri/pull/472))
- [docs] REFACTORING.md に地雷リストと互換パス sunset 条件を追記 (WP-S8 T2) ([#471](https://github.com/KingYoSun/kukuri/pull/471))
- [docs] 検証マトリクスの e2e-smoke 誤要求を実態へ修正 (WP-S8 T1) ([#470](https://github.com/KingYoSun/kukuri/pull/470))
- [docs] 視覚回帰の運用手順を dev.md に追加し検証マトリクスを更新 (WP-S7 T3) ([#469](https://github.com/KingYoSun/kukuri/pull/469))
- [scenario] 視覚回帰 baseline 14 枚を追加し CI で強制化 (WP-S7 T2) ([#467](https://github.com/KingYoSun/kukuri/pull/467))
- [scenario] 視覚回帰 spec + Playwright/xtask/CI 基盤配線(baseline なし)(WP-S7 T1) ([#466](https://github.com/KingYoSun/kukuri/pull/466))
- [contract] sqlite/memory backend parity ハーネス (WP-S6 T8) ([#465](https://github.com/KingYoSun/kukuri/pull/465))
- [fix] MemoryStore の sqlite との挙動乖離 2 件を修正 (WP-S6 T7) ([#464](https://github.com/KingYoSun/kukuri/pull/464))
- [contract] pagination の keyset 純関数と複数ページ走査を characterization (WP-S6 T6) ([#463](https://github.com/KingYoSun/kukuri/pull/463))
- [contract] row_mapping のエッジ/legacy 値を生 SQL で characterization (WP-S6 T5) ([#462](https://github.com/KingYoSun/kukuri/pull/462))
- [contract] row_mapping の put→get 全列 round-trip(16 写像)(WP-S6 T4) ([#461](https://github.com/KingYoSun/kukuri/pull/461))
- [contract] row_mapping の enum⇔文字列写像 14 関数を characterization (WP-S6 T3) ([#460](https://github.com/KingYoSun/kukuri/pull/460))
- [contract] migration の世代別 round-trip + 全適用後スキーマ golden (WP-S6 T2) ([#459](https://github.com/KingYoSun/kukuri/pull/459))
- [fix] migration down の可逆性を回復(20260329 補完 + 非可逆 down 3 件修正)(WP-S6 T1) ([#458](https://github.com/KingYoSun/kukuri/pull/458))
- Refactor/s5 t3 viewmodels harness ([#457](https://github.com/KingYoSun/kukuri/pull/457))
- [contract] renderHook 共通ハーネス + useDesktopShellViewModels の characterization (WP-S5 T3) ([#452](https://github.com/KingYoSun/kukuri/pull/452))
- [contract] selectors の未カバー主要 26 関数を characterization (WP-S5 T2) ([#451](https://github.com/KingYoSun/kukuri/pull/451))
- [contract] timelineMerge / routes 純関数の characterization テスト (WP-S5 T1) ([#450](https://github.com/KingYoSun/kukuri/pull/450))
- [fix] types.ts の過剰緩和 3 フィールドを実態へ修正 (WP-S4 T1) ([#444](https://github.com/KingYoSun/kukuri/pull/444))
- [fix] zh-CN の欠落 37 キーを補完 (WP-S4 T4) ([#445](https://github.com/KingYoSun/kukuri/pull/445))
- [refactor:extract] wire prefix の重複リテラルを core::wire へ集約(値不変) ([#447](https://github.com/KingYoSun/kukuri/pull/447))
- [docs] IPC codegen spike(ts-rs 12)の評価メモと WP-H7 判断 ([#449](https://github.com/KingYoSun/kukuri/pull/449))
- [docs] REFACTORING.md に「凍結境界」章を追加 ([#443](https://github.com/KingYoSun/kukuri/pull/443))
- [contract] CommunityNodeManifest の round-trip / wire golden (WP-S3 T6) ([#441](https://github.com/KingYoSun/kukuri/pull/441))
- [contract] moderation event の digest / issuer / 署名済み fixture golden ([#440](https://github.com/KingYoSun/kukuri/pull/440))
- [contract] rendezvous / replica / gossip id 派生の golden テスト ([#439](https://github.com/KingYoSun/kukuri/pull/439))
- [contract] GossipHint / posts の wire serde snapshot (WP-S3 T2) ([#438](https://github.com/KingYoSun/kukuri/pull/438))
- [contract] 署名 canonical 3 系統の golden テスト(envelope / DM frame / DM ack) ([#437](https://github.com/KingYoSun/kukuri/pull/437))
- [refactor:move] xtask: main.rs を機能別7モジュールへ分割 (WP-S2 T1) ([#431](https://github.com/KingYoSun/kukuri/pull/431))
- [fix] desktop-lint に no-console ルールを追加(warn/info のみ許可) ([#432](https://github.com/KingYoSun/kukuri/pull/432))
- [refactor:move] apps/desktop: DesktopShellPage.test.tsx をテーマ別15ファイルへ分割 (WP-S1 T1) ([#428](https://github.com/KingYoSun/kukuri/pull/428))
- [refactor:move] app-api: tests/mod.rs のヘルパを tests/support/ へ分割 ([#430](https://github.com/KingYoSun/kukuri/pull/430))
- [refactor:move] desktop-runtime: tests/mod.rs のヘルパを tests/support/ へ分割 ([#429](https://github.com/KingYoSun/kukuri/pull/429))
- @ ([#424](https://github.com/KingYoSun/kukuri/pull/424))
- dedupe Validation sections against Feature Data Classification ([#422](https://github.com/KingYoSun/kukuri/pull/422))
- decide ADR 0026 §6 open items (trust/relation, #416) ([#421](https://github.com/KingYoSun/kukuri/pull/421))
- non-deterministic (VLM) moderation ADR ([#411](https://github.com/KingYoSun/kukuri/pull/411), [#419](https://github.com/KingYoSun/kukuri/pull/419))
- deterministic moderation (CSAM / known-hash critical safety) ADR ([#410](https://github.com/KingYoSun/kukuri/pull/410), [#418](https://github.com/KingYoSun/kukuri/pull/418))
- community node trust / relation foundation ADR ([#409](https://github.com/KingYoSun/kukuri/pull/409), [#414](https://github.com/KingYoSun/kukuri/pull/414))
- community node indexing foundation ADR ([#412](https://github.com/KingYoSun/kukuri/pull/412))
- Add PLANS.md ([#401](https://github.com/KingYoSun/kukuri/pull/401))
- operator-config.yamlをgitignore ([#395](https://github.com/KingYoSun/kukuri/pull/395))
- critical safety docから不要な記述を削除 ([#387](https://github.com/KingYoSun/kukuri/pull/387))
- community node safety architecture を整備 ([#386](https://github.com/KingYoSun/kukuri/pull/386))
- README を最新の実装状況に更新 ([#379](https://github.com/KingYoSun/kukuri/pull/379))
- default community node 依存低減ロードマップを文書化 ([#360](https://github.com/KingYoSun/kukuri/pull/360), [#374](https://github.com/KingYoSun/kukuri/pull/374))
- community node shutdown と user continuity protocol を文書化 ([#361](https://github.com/KingYoSun/kukuri/pull/361), [#373](https://github.com/KingYoSun/kukuri/pull/373))
- moderation event / safety advisory を optional trust input として文書化 ([#362](https://github.com/KingYoSun/kukuri/pull/362), [#372](https://github.com/KingYoSun/kukuri/pull/372))
- P2P-first community node の責任境界を文書化 ([#354](https://github.com/KingYoSun/kukuri/pull/354), [#363](https://github.com/KingYoSun/kukuri/pull/363))
- 監査ベースの community-node ハードニングとユーザー入力検証 ([#350](https://github.com/KingYoSun/kukuri/pull/350))
- update CHANGELOG for v0.1.2-preview.1 ([#349](https://github.com/KingYoSun/kukuri/pull/349))

## [v0.1.2-preview.1] - 2026-06-15

### Features

- リリースごとのCHANGELOG自動生成・運用を追加 ([#342](https://github.com/KingYoSun/kukuri/pull/342), [#344](https://github.com/KingYoSun/kukuri/pull/344))
- topic一覧にsearch/filter/sort機能を追加 ([#340](https://github.com/KingYoSun/kukuri/pull/340), [#343](https://github.com/KingYoSun/kukuri/pull/343))
- topic/channelごとのGossip接続トグルを追加 ([#305](https://github.com/KingYoSun/kukuri/pull/305), [#341](https://github.com/KingYoSun/kukuri/pull/341))
- リポスト・リプライ・スレッドのUIを改善 ([#307](https://github.com/KingYoSun/kukuri/pull/307), [#337](https://github.com/KingYoSun/kukuri/pull/337))
- アプデ通知を改善（バナー廃止・更新時のみDLボタン表示・リリース設定を整理） ([#333](https://github.com/KingYoSun/kukuri/pull/333))
- add Japanese font fallback and a monospace token ([#328](https://github.com/KingYoSun/kukuri/pull/328))

### Fixes

- changelog ジョブを main 直push からPR作成方式へ変更 ([#348](https://github.com/KingYoSun/kukuri/pull/348))
- third-party notices のソートをオーディナル化しCI差異を解消 ([#347](https://github.com/KingYoSun/kukuri/pull/347))
- 接続エラー復帰後に community-node エラー表示が消えない問題を修正 ([#312](https://github.com/KingYoSun/kukuri/pull/312), [#335](https://github.com/KingYoSun/kukuri/pull/335))
- OS通知をRustバックエンド経由に変更しWindowsで発火するように修正 ([#313](https://github.com/KingYoSun/kukuri/pull/313), [#334](https://github.com/KingYoSun/kukuri/pull/334))
- unify shell breakpoints to the 759/899/900/1099/1100 system ([#331](https://github.com/KingYoSun/kukuri/pull/331))
- resolve undefined CSS custom-property references in shell styles ([#327](https://github.com/KingYoSun/kukuri/pull/327))

### Other

- regenerate third-party notices for 0.1.2 ([#346](https://github.com/KingYoSun/kukuri/pull/346))
- bump preview version to 0.1.2 ([#345](https://github.com/KingYoSun/kukuri/pull/345))
- AGENTS.local.mdを追加 ([#339](https://github.com/KingYoSun/kukuri/pull/339))
- @ ([#338](https://github.com/KingYoSun/kukuri/pull/338))
- codegraph導入 ([#336](https://github.com/KingYoSun/kukuri/pull/336))
- tokenize elevation, blur, and the metaverse canvas color ([#332](https://github.com/KingYoSun/kukuri/pull/332))
- tokenize spacing and radius into --space-* / --radius-* scales ([#330](https://github.com/KingYoSun/kukuri/pull/330))
- consolidate font-sizes into a --text-* type scale ([#329](https://github.com/KingYoSun/kukuri/pull/329))
- Rework DESIGN.md into a concrete visual design spec ([#326](https://github.com/KingYoSun/kukuri/pull/326))
- Update release readiness manual items ([#324](https://github.com/KingYoSun/kukuri/pull/324))
- Add startup database error screen ([#323](https://github.com/KingYoSun/kukuri/pull/323))
- Add store migration fixture ([#322](https://github.com/KingYoSun/kukuri/pull/322))
- Generate third-party notices ([#321](https://github.com/KingYoSun/kukuri/pull/321))
- Add updater error guidance ([#320](https://github.com/KingYoSun/kukuri/pull/320))
- Fix community node settings notice link ([#317](https://github.com/KingYoSun/kukuri/pull/317))

