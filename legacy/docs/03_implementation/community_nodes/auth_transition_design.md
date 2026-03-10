# relay/bootstrap 認証OFF→ON 切替設計（v1）

**作成日**: 2026年01月22日  
**対象**: `./kukuri-community-node`（relay / bootstrap / User API / Admin API/Console）

## 目的

- relay/bootstrap を **デフォルト認証OFF**で起動しつつ、管理画面から後から **認証必須化（ON）**できるようにする
- OFF→ON 切替時に、既存接続・猶予期間・互換性を明確化し、運用事故（想定外の開放/デッドロック）を避ける

## 前提（本計画のポリシー）

- relay/bootstrap が **認証OFF**の間は、ユーザー操作の手間を最小化するため **同意（ToS/Privacy）も不要**とする
- relay/bootstrap が **認証ON（必須）**になった場合は、pubkey を特定できるため **同意チェックを有効化**できる
  - 同意管理の詳細: `docs/03_implementation/community_nodes/policy_consent_management.md`

## 設定モデル（v1提案）

### 認証モード

- `auth_mode = off | required`
  - v1 は `optional` を持たない（互換性の曖昧さを避ける）。必要なら v2 で追加する

### 段階的な切替（必須）

OFF→ON は「設定変更の瞬間にいきなり締める」とクライアント側の再接続・再認証が間に合わず事故りやすい。  
そのため **施行時刻（予約）と猶予期間**を持つ。

- `enforce_at`: 新規接続に `auth_mode=required` を適用する時刻（例: 現在時刻 + 15分）
- `grace_seconds`: 既存接続が認証へ移行するための猶予（例: 15分）
  - `disconnect_unauthenticated_at = enforce_at + grace_seconds`
- `ws_auth_timeout_seconds`: relay WS で、接続後に AUTH を待つ最大秒数（例: 10秒）
  - 適用対象は「AUTH 必須状態で開始した接続（新規接続）」または「猶予なしで必須化された接続」
  - `enforce_at` 前から継続している既存接続は `disconnect_unauthenticated_at` を優先し、猶予中に `ws_auth_timeout_seconds` で切断しない

### 保管場所

- `cn_admin` のサービス設定として DB 永続化し、Admin Console から更新する
- 各サービスは DB をポーリング、または `LISTEN/NOTIFY` で変更を反映する（どちらでもよい）

## relay（WS）での挙動（v1）

### OFF（`auth_mode=off`）

- anonymous 接続を許可する
- 同意は不要（pubkey を特定できないため強制できない）
- ただし濫用対策（rate limit、接続数上限、`#t` 必須等）は常に有効

### ON（`auth_mode=required`）施行後

#### 新規接続

- 接続直後に NIP-42（AUTH）の challenge を提示し、`ws_auth_timeout_seconds` 以内に AUTH が無ければ切断する
- AUTH 成功後に以下を適用する
  - ToS/Privacy 同意チェック（未同意は `CONSENT_REQUIRED` 相当の通知を返し拒否）
  - user-level subscription（課金/権限）チェック（未契約/範囲外は拒否）

#### 既存接続（切替時の互換性）

- `enforce_at` までは現状維持（anonymous のままでも可）
- `disconnect_unauthenticated_at` 到来時点で「未AUTHの接続」は `NOTICE` を送って切断する
  - 目的: “購読だけ継続できてしまう穴”を残さない
- `ws_auth_timeout_seconds` は既存接続の猶予タイマーとしては使わない（責務分離）

#### 代表的な拒否シグナル（互換性）

Nostr クライアントは実装差があるため、複数の経路で「AUTH 必須」を伝える。

- publish（`EVENT`）: `OK false`（理由: `auth-required` 等）
- subscribe（`REQ`）: `CLOSED`（理由: `auth-required` 等）+ 必要なら `NOTICE`
- connection: `NOTICE` → close

補足:
- private scope（`scope!=public`）の WS 購読/backfill は、OFF の場合でも制御不能になりやすいので **常に AUTH 必須を推奨**する（詳細は `docs/03_implementation/community_nodes/access_control_design.md`）。

## bootstrap（HTTP）での挙動（v1）

### OFF（`auth_mode=off`）

- public として到達可能（同意不要）
- 返す内容は公開可能な範囲に限定する（public topic/公開 endpoint のみ等）

### ON（`auth_mode=required`）施行後

- `GET /v1/bootstrap/*` は認証必須にできる（`401` + `WWW-Authenticate`）
- 認証後は ToS/Privacy の同意チェックも有効化できる（未同意は `428` 等）

### デッドロック回避（推奨）

bootstrap を完全に閉じると「初回導線」が壊れやすい。運用上は次のどちらかを推奨する。

1. **最小 public bootstrap を残す**（例: `policy_url` と最小 descriptor のみ）
2. 完全クローズする代わりに、運用者が「配布URL/配布ファイル/既知ノード一覧」を別経路で配る

## Admin Console に出すべき設定（提案）

- `auth_mode`（off/required）
- `enforce_at`（即時/予約）
- `grace_seconds`（既存接続の猶予）
- `ws_auth_timeout_seconds`（relay の AUTH 待ち時間）
- 施行後の現在状態（`required` で未AUTH接続が残っていないか、拒否数、切断数）

## 関連ドキュメント

- `docs/03_implementation/community_nodes/services_relay.md`
- `docs/03_implementation/community_nodes/services_bootstrap.md`
- `docs/03_implementation/community_nodes/user_api.md`
- `docs/03_implementation/community_nodes/policy_consent_management.md`
- `docs/03_implementation/community_nodes/access_control_design.md`
