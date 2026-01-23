# Bootstrap サービス 実装計画

**作成日**: 2026年01月22日  
**役割**: discovery 補助（ヒント配布）+ node capability の広告

## 責務

- `node.descriptor(kind=39000)` / `node.topic_service(kind=39001)` の署名付き配布
- `scope`（public/friend/invite 等）を含む広告の管理（Access Control と整合させる）
  - 詳細: `docs/03_implementation/community_nodes/access_control_design.md`
- クライアントが初回接続時に必要とする bootstrap ヒント（既知ノード/エンドポイント）の配布
- 管理画面からの設定変更（対応 topic、ポリシーURL、管轄、連絡先等）の反映

## 外部インタフェース（提案）

- **HTTP（外部公開は User API に集約）**
  - `GET /v1/bootstrap/nodes`（node descriptor の一覧/差分）
  - `GET /v1/bootstrap/topics/:topic/services`（topic_service の一覧）
- **イベント（KIP）**
  - 39000/39001 を定期発行し、クライアントは gossip/DHT/既知URL 経由で収集できる

## 認証（デフォルトOFF / 後から必須化）

- デフォルトでは bootstrap の取得は **認証OFF**（public）とし、初回接続・発見の導線を壊さない
- 認証OFFの間は **同意（ToS/Privacy）も不要**とする（ユーザー操作の手間を最小化）
- 管理画面（Services）から `bootstrap` の **認証必須化**を切り替え可能にする
  - ON の場合、User API 側で「認証済み + 同意済み（ToS/Privacy）」を要求できる
  - OFF の場合でも、返す情報は **公開可能な範囲**（public topic/公開エンドポイント）に限定する
- 認証OFF→ON 切替の運用（予約/猶予/互換性、最小 public bootstrap の扱い）は `docs/03_implementation/community_nodes/auth_transition_design.md` を参照

## データ

- Postgres に「広告設定」（name, roles, endpoints, policy_url, jurisdiction, contact, topics）を保存
- 発行した event のメタ（発行時刻/次回期限）を保存（再発行管理）

補足:
- `policy_url` は `docs/03_implementation/community_nodes/policy_consent_management.md` の「公開URL」方針に沿って設定し、運用者が更新できるようにする。

## 実装手順（v1）

1. Node Key の読み込み/生成（volume に保管）
2. 39000/39001 の生成（必須 tag / `exp` / 署名）
3. HTTP API での配布（クライアントが取り込める最低限）
4. Admin API からの設定変更反映（DB → bootstrap）
5. rate limit（DoS 対策）と監査ログ（設定変更のみでも記録）
