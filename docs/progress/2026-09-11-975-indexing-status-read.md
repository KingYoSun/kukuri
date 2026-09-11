# #975 見つける検索が索引の反映状況を確認できるようにする（索引状況 read API と申請状態の表示）

- 判定: In progress（実装・ローカル検証完了。独立監査・CI・merge の最終結果は Issue の Current status と PR を参照する）
- Scope revision: `975-plan-v1 / 2026-09-11`（推奨案で承認済み）
- 基準 commit: `950bd48`
- リスク区分: C。community-node endpoint contract の追加、認証・同意境界、非公開チャンネル所属証明、client からの読取り経路。
- UI 分類: 既存画面の改善（申請 dialog、見つけるの空状態）。利用者は索引登録を申請した／検索が 0 件だった利用者。単一目的は、選択ノードでの索引状況（自分の申請状態、対象が索引対象か）を確定した情報として示し、未確認と区別すること。
- 対象外: 表示名・pubkey の索引化、自動承認、supported set 全体の一覧公開、他利用者の申請状態、harness scenario の追加（Optional-hardening）。
- 正本: [Issue 運用手順](../runbooks/issue-lifecycle.md)、[ADR 0025 §2.8](../adr/0025-community-node-indexing-foundation.md)、[ADR 0014](../adr/0014-uiux-dev-flow.md)、[DESIGN](../../DESIGN.md)、元 Issue #960 の[記録](2026-09-11-960-search-empty-state.md)。

## 調査で固定した事実

- `/v1/indexing/requests` は POST のみで、`pending / approved / rejected` は申請時応答にしか出ない。行は `cn_index.indexing_requests`（`UNIQUE (kind, target_id, requester_pubkey)`、`decided_at`）に残り、却下でも消えない。承認は同一 transaction で `supported_topics` へ入る。
- `requester_pubkey` で引く読取り関数は無かった（`list_indexing_requests` は status filter のみ。caller は `cn-cli` の一覧表示だけ）。
- 非公開チャンネルの所属証明は `require_channel_membership`（#711）が既にあり、未提示・不一致・未登録・鍵未設定を同一の 403 にまとめている。
- rate limit は router 全体の tower layer で、新 route に個別配線は不要。`tests/activation_gate.rs` の `SURFACES` が read 面ごとの失効コードを固定している。
- client は申請結果を dialog 内 `useState` にしか持たず、node 切替・再オープンで消える。空状態の文言と `mvp-troubleshooting.md` は「反映状況はアプリから確認できない」と明記していた。

## 契約（975-plan-v1）

- `GET /v1/indexing/status?scope_kind=&scope_id=`（両方あるか両方ないか）。gate は POST から抽出した `require_indexing_gate`（索引参照が構成済み → 準備完了記録が有効 → bearer → consent）。未構成・失効は認証より先に 404（`INDEXING_REQUEST_NOT_CONFIGURED` / `_NOT_ACTIVATED` を再利用）。
- 応答 `{ requests: [{ request_id, scope_kind, target_id, status, created_at, decided_at }], target: { scope_kind, scope_id, supported } | null }`。`requests` は呼出し主の行だけ。`target` は指定時のみ。非公開チャンネルの `target` は `x-kukuri-channel-secret` の所属証明を要求し、未証明は同一の 403。自分の申請一覧は所属証明なしで取得できる。
- client は状態を永続化しない。非公開チャンネルの secret は dialog の明示確認後の「索引状況を確認」でだけ送る。空状態からの自動取得は secret を伴わない（横断・非公開は一覧のみ、公開 topic は対象付き）。

## 固定した受入条件・不変条件

| ID | 条件 | 実装・証跡 |
| --- | --- | --- |
| AC-1 | 認証・同意済み利用者が自分の申請の現在状態を read endpoint から取得できる | `indexing_status_returns_own_requests_only_and_public_target_support`、`indexing_status_requires_auth_and_consent`（cn-user-api）。desktop-runtime `community_node_indexing_status_list_and_public_target_send_no_secret` |
| AC-2 | 非参加者に非公開チャンネルの索引有無を漏らさない | `indexing_status_private_target_requires_membership_proof`（未提示・不一致・未登録が同一 403、本文同一。非参加者の一覧は空） |
| AC-3 | 空状態と申請 dialog が確定した状態と未確認を区別して表示する | `CommunityIndexingRequestDialog.test.tsx`（14 件）、`CommunityIndexWorkspace.emptyState.test.tsx`（13 件）、`communityIndexEmptyGuidance.test.ts`、browser `community-index-empty-state.spec.ts` / `community-index.spec.ts` |
| AC-4 | 対象がそのノードの索引対象（supported set）かを取得できる。公開は認証・同意のみ、非公開は所属証明つき | 上記 cn-user-api 2 件、runtime `..._private_target_requires_confirmation_and_proof` |
| INVAR-1 | 多段ゲートと申請受付条件（#713）を変えない。POST の応答と既存 test を変えない | `indexing_status_is_hidden_when_index_is_not_configured_or_stale`、`activation_gate.rs` の `SURFACES` に追加。既存 POST test 9 件と `index_query.rs` 6 件は無変更で成功 |
| INVAR-2 | 状態読取りは `supported_topics` / `indexing_requests` / `channel_secrets` を書き換えない | 上記 cn-user-api test の `scope_state_counts` 前後比較（3 table 行数不変） |
| INVAR-3 | client は非公開チャンネル secret を明示確認なしに送らない。一覧取得は secret を伴わない。同意未承認・session 未確立では HTTP 前に停止する | runtime `..._list_and_public_target_send_no_secret`（ヘッダ None）、`..._private_target_requires_confirmation_and_proof`（確認なし／capability なしで HTTP 0 回）、`..._stops_before_http_when_session_is_not_ready`。dialog test「private target reads only own requests until the disclosure is confirmed」。workspace test（非公開 scope は `scope_kind: null`） |
| INVAR-4 | 状態を client に永続化しない | runtime は応答を返すだけ（store 書込みなし）。dialog / workspace は React state のみで、node 切替・再検索で取り直す（test「a stale status response does not overwrite …」、「再検索の空結果ごとに取り直す」、browser spec の再オープン） |

## Surface inventory（追加 INV-1〜7。既存入口の分類変更なし。未分類 0）

| ID | 入口 | helper | 読み書き・副作用 | guard | TR |
| --- | --- | --- | --- | --- | --- |
| INV-1 | `GET /v1/indexing/status` | `require_indexing_gate`（POST と共有）→ `require_bearer_identity` → `require_consents` | `indexing_requests` / `supported_topics` の SELECT のみ | 未構成／失効 404、401、403 | TR-1, TR-2 |
| INV-2 | 同 route の非公開 target | `require_channel_membership`（既存） | `channel_secrets` 復号・定数時間比較 | 未証明は同一 403 | TR-3 |
| INV-3 | `read_community_node_indexing_status`（runtime facade、Tauri command、CLI registry `Read`） | `ensure_community_node_session`、`load_community_node_token`、`private_channel_indexing_secret` | node へ GET（token、target 識別子、非公開では secret ヘッダ） | Ready 以外は HTTP 前に停止、非公開は確認 flag 必須 | TR-4 |
| INV-4 | dialog open / node 切替 | `api.readCommunityNodeIndexingStatus` | INV-3 を 1 回 | 公開は対象付き。非公開は一覧のみ | TR-5 |
| INV-5 | dialog の「索引状況を確認」（非公開、確認 checkbox ON） | 同上 | INV-3 を target 付きで 1 回。確認を 1 回消費 | 確認後のみ | TR-5 |
| INV-6 | 空状態（query 成功 0 件） | 同上 | INV-3 を 1 回（secret なし） | 既存の ready 条件 | TR-6 |
| INV-7 | mock（Storybook / browser test） | `mocks/api/connectivity.ts` | node 別の in-memory 申請簿 | なし | TR-5, TR-6 |

sensitive sink: server は DB 読取りと secret 復号（既存 helper）のみで、書込み sink は 0。client は「node への GET（bearer token = 利用者 identity、target 識別子）」と「非公開チャンネル secret のヘッダ送信」の 2 つ。後者は同意 Ready、capability 保持、明示確認 flag の 3 条件が制御フロー上で先行する（`validate_status_target` → `ensure_community_node_session` → `private_channel_indexing_secret`）。

## 状態遷移

| ID | 事前状態 / sequence | 期待状態 | 禁止する副作用 | 検証 |
| --- | --- | --- | --- | --- |
| TR-1 | 認証・同意済み → 申請なし → GET → POST → 承認 → GET | `requests` 空 → pending → approved、`supported` false → true | GET による行の追加・更新 | cn-user-api |
| TR-2 | 未構成 / 失効 / 未認証 / 未同意 → GET | 404（認証より先）/ 401 / 403 | token・consent の更新 | cn-user-api、activation_gate |
| TR-3 | 非公開 target を 未提示 / 不一致 / 未登録 で GET | 同一 403、同一本文 | 存在有無の差 | cn-user-api |
| TR-4 | 同意保留 / retrying session / capability なし / 確認 flag なし → runtime | HTTP 0 回で typed error | secret の組立て・送信 | desktop-runtime |
| TR-5 | dialog open → 取得中 → 応答 / 失敗 / node 切替 / 再 open | 確定表示 / 未確認表示 / 古い応答の破棄 | 取得失敗による送信不能、状態の永続化 | Vitest、browser |
| TR-6 | 空状態 → 状態取得 → error / 429 / node 切替 | 空状態と状態表示が同時に消える | 取得失敗の「索引対象外」への断定 | Vitest、browser |

## 作業

- T1: ADR 0025 に §2.8（read 面。ADR 0002 分類含む）と必須 contract 3 件を追記。`docs/legal/app-data-flow-inventory.md`、`external-transmission-notice.md`（version 5 補記。版・施行日は変更しない）、i18n `legal` 3 locale に「索引状況の確認」を追加。
- T2: `cn-protocol` に `INDEXING_STATUS_PATH`、`IndexingStatusParams`、`IndexingRequestView`、`IndexingTargetStatus`、`IndexingStatusResponse` と wire contract test。
- T3: `cn-core` に `list_indexing_requests_for_requester`（他人の行を含まず、却下行を含む）。
- T4: `cn-user-api` の POST から `require_indexing_gate` を抽出し、`indexing_status` handler を追加。`parse_index_scope_params` を `parse_scope_pair` に一般化。`IndexingOperation::ReadStatus`。contract test 4 件と `SURFACES` 追加。
- T5: desktop-runtime `indexing_status_support.rs`（申請と同じ順序、401 再認証 1 回、error 型は `CommunityNodeIndexingRequestError` を共有）、facade、Tauri command、`kukuri-cli` dispatcher / registry / schema / `command-parity.json`（144 件）、`cargo xtask ipc-types`、`types.ts` / `runtimeApi.ts`、mock の node 別申請簿。runtime test 4 件は `tests/community_node/indexing_status.rs` へ分離（`index_query.rs` を 1000 行未満に保つ）。
- T6: dialog は open / node 切替で状態を読み、loading / 確定 / 未確認（理由 + 再確認）を区別。申請済み・索引対象の対象は送信を無効化して理由を表示。非公開は一覧のみ読み、確認後の「索引状況を確認」で対象付きに読む（確認を 1 回消費）。申請応答は再取得せず表示 state へ反映。i18n `indexingRequest.indexStatus.*`、`errors.{membershipRequired,capabilityUnavailable,statusFailed}`。stories 2 件追加。
- T7: `communityIndexEmptyGuidance` に `indexStatus` 入力を追加し、確定状態で理由を断定文へ（`notSupportedTarget` / `requestPending` / `requestRejected`）。申請済み・索引対象では申請 CTA を出さない。横断は自分の申請を状態別に表示。`CommunityIndexWorkspace` は空結果ごとに同じ node へ 1 回読む（公開 topic は対象付き、横断・非公開は一覧のみ）。`notIndexedYet` の文言から「アプリから確認できない」を外し、「個々の投稿の反映状況は確認できない」に改めた。`mvp-troubleshooting.md` を更新。stories 4 件、browser spec 更新。

## 実行結果（Linux container、Node 22、pnpm 10.16.1。Postgres 16 / Redis 7 を apt で導入してローカル起動）

| 検証 | 結果 |
| --- | --- |
| `cargo test -p kukuri-cn-protocol --test index_contract` | 6 件成功 |
| `cargo test -p kukuri-cn-core --test index_scope`（`KUKURI_CN_RUN_INTEGRATION_TESTS=1`） | 9 件成功 |
| `cargo test -p kukuri-cn-user-api --test indexing_requests --test activation_gate --test index_query --test contract_auth --lib` | indexing_requests 13、activation_gate 7、index_query 6、contract_auth 18、lib 35 件成功（contract_auth の初回失敗は disk full による Postgres 停止が原因で、再起動後に全件成功） |
| `cargo clippy -p kukuri-cn-protocol -p kukuri-cn-core -p kukuri-cn-user-api -p kukuri-desktop-runtime -p kukuri-cli --all-targets -- -D warnings` | 成功 |
| `cargo test -p kukuri-desktop-runtime --lib community_node::index_query` / `community_node::indexing_status` | 14 件 + 4 件成功 |
| `cargo test -p kukuri-cli --test command_parity --test community_node --test daemon_linux` | 5 件 + 2 件 + 8 件成功（`daemon_linux` の registry 総数 assertion は CI の初回失敗で検出し 135 → 136 へ更新） |
| `cargo xtask ipc-types` | 再生成（差分は新 4 型のみ。生成器由来の行末空白は既存行と同じ形式） |
| `cargo xtask oversized-files` | 成功（test 分離後） |
| desktop `pnpm lint` / `typecheck` | 成功 |
| desktop Vitest 対象 suite（dialog 14、emptyState 13、guidance、workspace、consentGate、shell communityIndex、i18n parity、runtimeApi） | 成功 |
| desktop Vitest 全体 | `DesktopShellPage.{columnScope,columnParents,channels}.test.tsx` の 4 件が 5 秒 timeout。基準 commit（stash）でも同じ 2 suite が同様に timeout することを確認し、本変更由来ではない環境依存として扱う。他は成功 |
| Storybook build | 成功 |
| Playwright chromium（`community-index-empty-state` 7、`community-index-layout` 16、`community-index` 3） | 26 件成功。repo 固定の chromium build が container に無いため、`/opt/pw-browsers/chromium` を `executablePath` に指定する未 commit の local config で実行 |
| `cargo xtask tauri-check` 相当 | 未実行。container に `gdk-3.0` 等の Tauri system 依存が無く compile できない。CI の tauri-check で確認する |
| `cargo xtask cn-test` 全体 | 未実行。container のディスク割当てにより test binary 群を同時に build できず、対象 test を個別に実行した |

### 未確認の境界

- Tauri 実機（Ubuntu / Windows WebView2）での表示・focus は未確認。remote container のため実 App を起動できない。
- 実 CN（ArcadeDB 込み）で承認後の検索結果まで通す E2E は未実施。`supported` の判定は `cn_index.supported_topics` の読取りで固定し、投稿の反映は既存の fail-closed gate に委ねる。
- Linux visual baseline は変更なし（空状態・dialog は既存 visual spec の撮影対象外）。

## 追加発見の分類

- Existing-gap: なし。
- Regression: `community-index.spec.ts` の「申請後に確認し直すと再申請できる」期待は、承認済み判断 1（申請済み対象は再申請を塞ぐ）による意図した変更で、spec を新契約に合わせた。
- New-requirement: 非公開チャンネルの `supported` を空状態から読む（所属証明の自動送信）は対象外のまま。必要なら別 Issue。
- Optional-hardening: harness scenario への状態読取り step 追加。

## 独立監査（2026-09-11、別コンテキスト）

- 対象 commit: `9c8fb7d`（PR #985 head）
- Scope revision: `975-plan-v1 / 2026-09-11`
- リスク区分: C
- inventory: 合計 14 / 適合 14 / 不適合 0 / 未分類 0（入口 8: route 登録、runtime facade、Tauri command、CLI registry + parity + schema、`runtimeApi`、mock、dialog の 4 呼出し、workspace の空状態 effect。sink 6: `require_indexing_gate`、`parse_scope_pair`、`require_channel_membership`、`list_indexing_requests_for_requester` + `is_topic_supported`、secret ヘッダ付与 + `private_channel_indexing_secret`、token 読取り + 401 再認証 1 回）
- AC / INVAR evidence: AC-1〜4、INVAR-1〜4 すべてに実装 symbol と test を対応付け、監査者自身の実行で成功を確認（詳細は PR #985 の監査 comment）
- 実行した validation: cn-user-api `indexing_requests` 13 / `activation_gate` 7 / `index_query` 6、desktop-runtime `indexing_status` 4、cn-core `index_scope` 9、cn-protocol `index_contract` 6、kukuri-cli `command_parity` 5、Vitest 4 file 62 件。TR-1〜6 をコード読みで照合
- blocker: 0件
- non-blocker とした事項: (1) Optional-hardening: GET 専用の「鍵未設定」test は無く、共有 helper の分岐と POST 側 test で担保。(2) Optional-hardening（doc 整合）: ADR §2.8 の test 参照が実名・実 file と食い違っていた → 監査後 delta として docs のみ修正。(3) Optional-hardening: 空状態は own request `approved` を `supported` 実値より優先表示するため、承認後に operator が supported から外した稀なケースで文言が食い違い得る（dialog は両者を別表示、再申請は冪等）。(4) Existing-gap（#698 由来、本 diff で不変）: 適格一覧の内容変化で dialog が選択 node を先頭へ戻す。
- 判定: PASS
- 監査後 delta: ADR 0025 §2.8 の test 参照修正（docs のみ）。CI `linux-rust-tests` の失敗を受けて `crates/kukuri-cli/tests/daemon_linux.rs` の registry 総数 assertion を 135 → 136 へ更新、desktop-runtime `tests/support/lock_contract.rs` の lock 分類表に `indexing_status.rs` の `CommunityNodeServer` 取得 4 件を宣言し合計を 128 → 132 へ更新（いずれも test の期待値のみ。監査対象の入口・sink・guard に変更なし）
