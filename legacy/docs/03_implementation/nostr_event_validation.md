# Nostr イベント検証ポリシー
最終更新日: 2025年10月30日

## 1. 概要
- 対象: kukuri 内で取り扱う Nostr イベント（P2P/Gossip 経由・Tauri コマンド経由・Offline リプレイを含む）。
- 目的: NIP-01/10/19 および kind:30078（Parameterised Replaceable Event）の準拠条件を明文化し、実装・テスト・運用の一貫性を確保する。
- 適用範囲: アプリケーション層（`EventGateway`/`EventService` 等）が実施する検証、ならびにインフラ層が担う早期遮断（JSON/署名）の責務。

## 2. レイヤ責務
- **インフラ層 (`infrastructure::p2p::IrohGossipService` など)**  
  - 受信パケットの JSON デコード、署名検証（失敗時は即時破棄）。  
  - 破棄理由を `tracing` とメトリクスに記録し、上位へは詳細データを渡さない。
- **アプリケーション層 (`EventGateway`/`EventService`/`OfflineService`)**  
  - 本ドキュメントで定義する NIP 準拠チェックを実施し、結果に基づくリトライ・通知・隔離処理を司る。  
  - 失敗時は `AppError::ValidationError` を発行し、再索引対象から除外する判断を下す。
- **テスト層 (`tests/contract` / `tests/integration`)**  
  - JSON フィクスチャおよび統合シナリオで Pass/Fail 条件を検証し、仕様逸脱を検知する。

## 3. 検証マトリクス（Pass/Fail）
| 対象 | Pass 条件 | Fail 条件 |
| --- | --- | --- |
| **NIP-01 基本整合性** | `id`/`pubkey`/`sig` が 64/64/128 桁の hex、`id` 再計算一致、`created_at` が `現在 ±10分` 以内、JSON スキーマ妥当 | hex 形式不正、署名再計算不一致、タイムスタンプ乖離、シリアライズ失敗 |
| **NIP-10 返信タグ** | `e`/`p` タグが 64hex または bech32（`note`/`nevent`/`npub`/`nprofile`）、`relay_url` は空 or `ws[s]://`、`marker` は `root`/`reply`/`mention` のみ、`root`/`reply` はそれぞれ高々 1 件（`reply` 単独は互換モードで許容） | marker 未定義、`relay_url` が http(s) 以外、`root`/`reply` 重複、bech32 不整合 |
| **NIP-19 bech32 TLV** | `npub`/`nprofile`: tag=0 が 32byte 公開鍵、relay tag ≤16・ASCII・`ws[s]://`。`nevent`: tag=0=event ID、tag=2=author(32byte 任意)、tag=3=kind(4byte BE)。 | TLV 長超過、非 ASCII、relay 上限超過、`hrp` 不一致、未定義 tag が 1KB 超 |
| **kind:30078 PRE** | `kind`=30078、`["d","kukuri:topic:<slug>:post:<revision>"]` 必須（`slug` は `TopicId` 準拠、`revision` は base32 26文字またはハイフン無し UUID）、`["k","topic-post"]` 固定、`["t","topic:<slug>"]` 単一指定、`["a","30078:<pubkey>:kukuri:topic:<slug>:post:<revision>"]` 一致、`content` は JSON `{body,attachments,metadata}` で 1MB 未満、timestamp が最新（同一時刻は `id` 比較） | `d` 欠如/形式不正、`k`/`t`/`a` 欠落または不一致、複数トピック指定、`content` サイズ超過/JSON 不正、古い timestamp が最新を上書き |
| **共通制限** | `content` ≤ 1MB、`tags` ≤ 512、内容が UTF-8 妥当 | サイズ超過、非 UTF-8、未知タグによる重大フォーマット崩れ |

### 3.1 kind:30078 の詳細仕様
- **`d` タグ**: `kukuri:topic:<slug>:post:<revision>` 形式。`slug` は `TopicId`（`[a-z0-9-]{1,48}`）をベースに URL-safe 化した文字列、`revision` は 26 文字の Crockford base32 もしくはハイフンの無い UUIDv7 を推奨。  
- **`k` タグ**: `["k","topic-post"]` 固定。将来のサブタイプ拡張のため識別子として利用する。  
- **`t` タグ**: 必須で 1 件限定。値は `topic:<slug>`。複数トピック横断投稿は未サポート。  
- **`a` タグ**: `["a","30078:<pubkey>:kukuri:topic:<slug>:post:<revision>"]` を必須化し、PRE の完全キーとする。  
- **`content`**: UTF-8 JSON オブジェクトで以下のフィールドを持つ。  
  ```json
  {
    "body": "投稿本文",
    "attachments": ["iroh://..."],
    "metadata": {
      "app_version": "2.5.0",
      "edited": false
    }
  }
  ```
  - `body`: 文字数は 1MB 未満。改行と絵文字は可。  
  - `attachments`: 0〜16 件。各要素は `iroh://` もしくは `https://` で始まる ASCII URL。  
  - `metadata.app_version`: セマンティックバージョン文字列。  
  - `metadata.edited`: PRE 上書き時に `true` を設定。  
- **PRE 上書き優先順位**: `created_at` が新しい方を採用。同一タイムスタンプの場合は `EventId` の lexicographical 比較で勝者を決定する。敗者は `nostr_event_validation::Precedence::Rejected` としてログに出力し、Offline 再索引で破棄する。  
- **返信・引用**: NIP-10 に従い、`["e",<event_id>,"","reply"]` および `["e",<event_id>,"","mention"]` を併用。互換性のため `reply` 単独も許容するが、`root` を併記することを推奨する。メンション対象には `["p",<pubkey>]` を追加する。  

## 4. 実装ポイント
1. `kukuri-tauri/src-tauri/src/domain/entities/event.rs`  
   - `validate_nip01` にタイムスタンプドリフトチェックと hex 厳格判定を追加する。  
   - `validate_nip10_19` に relay URL、marker、bech32 TLV の詳細検証を実装する。  
   - kind:30078 専用バリデータを追加し、`d`/`k`/`t`/`a` の整合性、`content` JSON Schema、PRE 上書きルールを確認する。
2. `infrastructure/p2p/iroh_gossip_service.rs`  
   - JSON パースおよび署名検証で失敗したイベントを早期に破棄し、`record_receive_failure()` へ伝播する。  
   - アプリケーション層へはバリデーション結果に応じた `AppError` を返却する。
3. `application/services/event_service`  
   - `EventGateway` での検証結果に基づき、UI 通知・リトライ・隔離処理（例えばオフラインキューへの登録）を制御する。

## 5. テスト戦略
- **契約テスト (`kukuri-tauri/src-tauri/tests/contract`)**  
  - `nip10_contract_cases.json` を拡張し、NIP-10 の Pass/Fail ケースを管理。  
  - 新規に `nip19_contract_cases.json`、`kind30078_contract_cases.json` を追加し、bech32/TLV/identifier/JSON Schema の境界値を網羅する。
  - JSON フィクスチャ方式を採用し、`case_id` / `description` / `input` / `expected` を統一フォーマットで管理する。差分レビューがしやすいよう、ケース追加時は ID を連番で付与する。
- **統合テスト (`kukuri-tauri/src-tauri/tests/integration`)**  
  - `p2p_*_smoke` に不正イベントの送信シナリオを追加し、P2P 経路での検証と破棄を確認する。  
  - Offline 再索引 (`tests/integration/offline`) に過去イベントの再検証ケースを追加し、旧フォーマットを除外できるか確認する。
- **メトリクス検証**  
  - `scripts/metrics/export-p2p.*` で `receive_failures` が増加していないか定期チェック。  
  - 失敗が検知された場合はログ上で `validation_error` の内容を確認し、フィクスチャ更新が必要か判断する。

### 5.1 テストマッピング
| レイヤ | ファイル/場所 | 対応ケース | 備考 |
| --- | --- | --- | --- |
| ドメインユニット | `domain/entities/event.rs` 内 `#[cfg(test)]` | NIP-01 基本/異常、NIP-10 marker・relay、NIP-19 TLV、kind30078 PRE 上書き | バリデータ関数の純粋ロジック検証 |
| 契約テスト | `tests/contract/nip10.rs`、新規 `nip19.rs`、`kind30078.rs` と JSON フィクスチャ群 | 公開 NIP サンプルと kukuri 拡張の Pass/Fail | 仕様変更時にフィクスチャを更新 |
| P2P 統合 | `tests/p2p_gossip_smoke.rs`、`tests/p2p_mainline_smoke.rs`、新規 `tests/p2p_kind30078.rs` | 受信ドロップ、PRE 最新採用、メトリクス増減 | Docker/CI で `ENABLE_P2P_INTEGRATION=1` で実行 |
| Offline 統合 | `tests/integration/offline/recovery.rs` ほか | 再索引時の再検証、PRE の旧 revision 排除 | OfflineReindexJob が最新のみを復元することを確認 |
| フロントユニット | `src/tests/unit/hooks/useNostrEvents.test.tsx` 等 | kind:30078 イベント処理、編集フラグ | 受信後の UI 更新フローを担保 |
| メトリクス/監視 | `scripts/metrics/export-p2p.*`、`docs/03_implementation/p2p_mainline_runbook.md` | `receive_failures`、PRE リジェクト件数 | 本番運用での逸脱検知に利用 |

## 6. 運用手順
1. ローカルまたは CI でテスト実行後、`docs/03_implementation/p2p_mainline_runbook.md` 9章の条件に照らし、ログとメトリクスで準拠状況を確認する。  
2. 既存データベースで古いイベントが検出された場合は、オフライン再索引ジョブを実行し、kind:30078 の上書き方針（最新優先）が適用されているか確認する。  
3. 仕様変更が発生した場合は、本ドキュメントと Runbook を更新し、契約テストフィクスチャも同時に見直す。

## 7. 参考リンク
- `docs/03_implementation/p2p_mainline_runbook.md` 9章（運用要約）  
- `docs/01_project/activeContext/iroh-native-dht-plan.md` 9章（残タスクと検証方針）  
- NIP-01, NIP-10, NIP-19, NIP-30078 (Parameterised Replaceable Events) 公式仕様

## 6. メトリクスとログ／オフライン整合
1. **メトリクス拡張**  
   - `ValidationFailureKind` enum を導入し、`record_receive_failure_with_reason(kind)` で `receive_failures_by_reason` を集計する。  
   - `P2PMetricsSnapshot` に理由別カウンタを追加し、P2PDebugPanel と Runbook から確認できるようにする。  
2. **ログ方針**  
   - 検証失敗時は `tracing::warn!(reason = %kind, event_id = %event_id, event_kind = %event_kind, "dropped invalid nostr event")` を基本とする。  
   - 同一 `reason` が 60 秒以内に 3 回以上発生した際は DEBUG へレベルを下げるレートリミットを設け、ノイズを抑制する。  
3. **終端処理**  
   - P2P 受信経路では不正イベントを永続化せず破棄し、`AppError::ValidationError(ValidationFailureKind)` をアプリケーション層へ返却する。  
   - Offline 再索引で同イベントを検出した場合は `SyncStatus::Invalid(kind)` として隔離し、`OfflineReindexReport` に理由を含める。  
4. **整合性検証**  
   - `offline::reindex` 系テストで `Invalid` 判定がレポートに反映され、同じイベントが再度キューへ載らないことを検証する。  
   - Runbook 第9章に、メトリクスとレポートで理由別件数を確認する手順を追記する。

## 5. 実装ステータス（2025年10月30日）
- ドメイン: `Event::validate_for_gateway` を新設し、NIP-01/10/19 および kind:30078 の検証を一元化。kind:30078 用のタグ検証と JSON スキーマ検証は `validate_kind30078` 系ヘルパーで 16 件の添付制限・Crockford base32/hex 判定・SemVer チェックまで実装済み。
- 共通エラー: `shared::validation::ValidationFailureKind` と `AppError::ValidationError` を拡張し、P2P 受信・Offline 同期・DTO バリデーションが同じ理由コードを返すよう統合。`SyncStatus::Invalid(<kind>)` で再索引結果に理由文字列を保持する。
- メトリクス/ログ: `infrastructure::p2p::metrics` に `receive_failures_by_reason` を追加し、`EventManagerGateway`/`iroh_gossip_service` で失敗理由をカウント。60 秒ウィンドウ内 3 回超過で WARN→DEBUG へレートリミットする `log_validation_failure` を導入。
- 契約/ユニットテスト: `domain/entities/event.rs` に NIP-10/19/kind30078 の境界テストを追加、`tests/contract/nip10.rs` の JSON ケースを最新仕様に揃えて `cargo test --test contract` で確認。`reply` 単独は互換モードとして許容し、契約テストと実装の整合を維持。
- テスト実行: `cd kukuri-tauri/src-tauri && cargo fmt && cargo test` を完走し、ドメインユニット・契約・統合（P2P/Offline）全てで Validation 変更が後方互換であることを確認済み。
