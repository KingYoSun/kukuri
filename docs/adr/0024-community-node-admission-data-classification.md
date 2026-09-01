# ADR 0024: Community-Node Admission (Invite / Whitelist / Ban) Data Classification

## Status
Accepted

## Feature Data Classification
- Feature 名: community-node admission (invite code / whitelist / ban)
- Durable / Transient: Durable server state
- Canonical Source: server `Postgres`（`cn_admin.service_configs` の admission mode、`cn_admin.invite_codes`、`cn_admin.admission_allowlist`、`cn_user.subscriber_accounts.status`）
- Replicated?: server の admission state は client へ replicate しない。client が入力した招待コードも端末間同期しない
- Rebuildable From: `Postgres` migrations + 運営者が cn-cli で投入した admission state
- Public Replica / Private Replica / Local Only: server の運用データは private server state のみ。client が入力した招待コードは node の正規化済み `base_url` を鍵にした Local Only の秘密情報として keyring（利用不能時は権限を制限した秘密ファイル）へ保存する。public manifest、通常の community-node 設定、状態取得、診断、ログには平文を載せない
- Gossip Hint 必要有無: No
- Blob 必要有無: No
- SQLite projection 必要有無: No（server は SQLite を使わない）
- 必須 contract:
  - `admission_open_mode_admits_new_subscriber`
  - `admission_invite_mode_admits_with_valid_code`
  - `admission_invite_mode_requires_code`
  - `admission_invite_mode_rejects_invalid_code`
  - `admission_invite_mode_rejects_expired_code`
  - `admission_invite_mode_rejects_exhausted_code`
  - `admission_invite_mode_rejects_revoked_code`
  - `admission_invite_mode_allowlisted_pubkey_bypasses_code`
  - `admission_whitelist_mode_admits_allowlisted`
  - `admission_whitelist_mode_rejects_unlisted`
  - `admission_pre_ban_rejects_verify`
  - `admission_existing_active_subscriber_passes_in_invite_mode`
  - `admission_banning_active_subscriber_revokes_existing_token`
- 必須 scenario:
  - なし（admission は server-side enforcement であり、現行 harness の `community_node_public_connectivity` は default `open` で不変通過することを回帰として扱う）

## Decision
- public community node の利用者を限定する admission を server-side（`cn-core` enforcement + `cn-user-api` surface + `cn-cli` 運営）に固定する。
- node 全体の入会モード（`open` / `invite` / `whitelist`）は `cn_admin.service_configs` の service `community_node_admission` に runtime 可変 state として持つ。未設定時は `open` を seed し、既存挙動と後方互換を保つ。
- 招待コードはハッシュ（SHA-256）で `cn_admin.invite_codes` に保存し、平文は保存しない。`max_uses`（NULL=無制限）/ `expires_at`（NULL可）/ `revoked_at` を持ち、redeem は同一トランザクション内の条件付き `UPDATE ... RETURNING` で原子的に消費する。
- whitelist は `cn_admin.admission_allowlist`（pubkey 主キー）で表現する。
- ban は `cn_user.subscriber_accounts.status='banned'` で表現する。未登録 pubkey の事前 ban は banned 行を upsert する。
- admission enforcement の適用順序は次に固定する。
  1. `status='banned'` は mode に関わらず拒否する（既存トークンも `require_bearer_identity` の status 再チェックで即時失効する）。
  2. 既存 `status='active'` subscriber は mode 変更後も再認証を通す（運用変更で既存利用者を突然締め出さない）。
  3. それ以外（未登録 pubkey）のみ mode を適用する（`open`=admit / `whitelist`=allowlist 登録のみ / `invite`=有効コード必須、ただし allowlist 該当はコード不要 bypass）。
- 拒否は `POST /v1/auth/verify` で HTTP 403 + 専用コード（`INVITE_REQUIRED` / `INVITE_INVALID` / `INVITE_EXPIRED` / `INVITE_EXHAUSTED` / `INVITE_REVOKED` / `NOT_ALLOWLISTED` / `BANNED`）として返す。
- client は HTTP 403 の応答本文を破棄せず、上記の安定コードと message を保持する。招待コードは入力対象の node にだけ送信し、別 node へ転用しない。
- client は admission 拒否を一時的な通信失敗と区別し、自動再試行を止めて利用者対応待ちにする。招待関連の拒否はコードの入力または再取得を案内し、`NOT_ALLOWLISTED` と `BANNED` は node 運営者への連絡を案内する。この扱いは配布候補か利用者追加 Node かで変えない。

## Consequences
- admission は node-local な「補助機能提供の可否」判断である。`docs/architecture/p2p-first-community-node-responsibility-boundary.md` の責任境界に従い、ban は kukuri network 全体からのアカウント凍結ではなく、この node が提供する接続補助・auth/consent をこの pubkey へ提供しないという node-local な制限として扱う。user identity / profile / social graph は node-independent であり admission の対象にしない。
- `ensure_database_ready` の必須テーブルに `cn_admin.invite_codes` と `cn_admin.admission_allowlist` を加えるため、prepared DB 前提の本番起動は migration 未適用時に fail-fast する（標準入口は `cn-cli prepare` / `cn-migrate`）。
- 課金ゲート（payment gate）は本 ADR のスコープ外だが、admission mode と enforcement の拡張点として将来差し込める構造とする。
- client は node ごとの招待コード入力導線、保存済み表示、安定コードに対応した理由と次の操作を提供する。秘密値そのものは UI 状態へ戻さず、保存有無だけを返す。
