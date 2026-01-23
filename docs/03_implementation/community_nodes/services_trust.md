# Trust サービス（Apache AGE）実装計画

**作成日**: 2026年01月22日  
**役割**: attestation 発行、集約（収益化ポイント）

## 前提（要件）

- trust 計算は Apache AGE（Postgres 拡張）を用いる
- trust の計算方式は 2 種類を用意する
  1. **通報ベース**
  2. **ユーザーごとのコミュニケーション濃度ベース**

## 出力（KIP-0001 寄り）

- `attestation(kind=39010)` を署名して配布する（スコアの押し付けではなく“主張”）
- `exp` を付与し、固定化/永続BAN を避ける（暫定判断として扱えるようにする）
- 取込レコードは relay が Postgres に保存したものを入力として扱い、outbox を `seq` で追従する（`consumer_offsets` による offset 管理、at-least-once を冪等処理で吸収）
  - 詳細: `docs/03_implementation/community_nodes/outbox_notify_semantics.md`

## イベント削除/期限切れの扱い（v1）

- 置換/削除/期限切れ等の扱いは relay が統一し、trust はその結果（有効なイベント/削除通知）に追従する
  - 詳細: `docs/03_implementation/community_nodes/event_treatment_policy.md`
- v1 の推奨:
  - report-based: 削除に影響させない（通報履歴は残す）
  - communication-density: 削除でエッジを消さず（ゲーム耐性）、ただし表示/根拠提示からは除外できるようにする

## 方式A: 通報ベース trust（v1）

### 入力
- report（39005）
- moderation label（39006）（あれば重み付けに使用）

### 計算（例）

- 期間窓（例: 7日/30日）で report を集計し、reason ごとに重みを付けて `risk_score` を算出
- reporter の信頼度（方式Bなど）で report の重みを補正できるようにする（v2）

## 方式B: コミュニケーション濃度ベース trust（v1）

### 入力（例）

- public な相互作用（reply/mention/reaction 等）から「相互作用グラフ」を構築
- 暗号化領域（friend/invite）のデータは、原則として trust の入力にしない（プライバシー保護）

### 計算（例）

- 時間減衰（recent を強く）+ 相互作用種類の重みで `INTERACTED` エッジ重みを更新
- ユーザーごとの中心性/局所密度を指標化（v1 は単純な重み合計/正規化から開始）

## 外部インタフェース（提案）

- 外部公開は User API に集約する
  - `GET /v1/trust/report-based?subject=pubkey:...`
  - `GET /v1/trust/communication-density?subject=pubkey:...`
- 管理者操作は Admin API 経由
  - `POST /v1/attestations`（手動/再計算/再発行）

## 実装手順（v1）

1. AGE 初期化（graph 作成、最小の vertex/edge モデル）
2. 入力取り込み（reports + interactions）
3. 2方式でスコア算出（まずは単純な集計）
4. `attestation(39010)` 生成・署名・配布
5. 計算ジョブ管理（キュー/進捗/失敗）を Postgres に記録
