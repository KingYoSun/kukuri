# Project Arachnid Shield Integration (Community Node Operator)

最終更新日: 2026-07-02

## 目的

Community Node operator が **自分の** Project Arachnid Shield account / API credentials を使って、
kukuri の known-CSAM safety provider（`project-arachnid-shield`、Issue #391）を設定・検証・運用する
ための手順。

kukuri can be configured by operators to integrate with Project Arachnid Shield using their own
credentials. kukuri 本体は credentials を同梱・共有・代理利用しない。

> 注意: この runbook は法的助言ではない。CSAM への対応は各国法上の削除・通報義務を伴う。
> Shield の利用はそれらの義務や、IHC / 警察等への通報フロー整備を**代替しない**。通報フローは
> operator が自身の管轄法に基づき別途整備すること。

## 前提と責任分界

- **credentials は operator 所有**: 各 operator が [projectarachnid.com](https://projectarachnid.com)
  で自分の account を取得する。kukuri 公式は shared / demo key を提供しない。hosted facilitator が
  third-party operator の key を代理利用することも禁止。
- **Shield API の利用は operator 自身の責任**で、Project Arachnid の Terms and Conditions of Use に
  従う。
- 自分のサービスで Shield を利用中であることを**公表する場合**は、Project Arachnid Terms の
  public statement 条項に従い、必要に応じて指定文言または事前承認を使う。kukuri の docs / UI では
  `certified` / `approved` / `officially supported` / `protected by` 等の表現を使わない。

## Shield へ送信されるデータ（Submitted Data）

| 経路 | 送信内容 | 使用箇所 |
|---|---|---|
| `POST /v1/media` | media 本体（bytes）| runtime の media scan（media fetcher 接続後） |
| `POST /v1/pdq` | PDQ hash（32 bytes の base64。media 本体ではない） | `safety test-provider` の connectivity check |
| `POST /v1/url` | media を指す URL | **未実装**（将来。account の Authorized Domains 登録が必要） |

- Community Node は blob 本体を恒久保存しない（`safety.storage.permanent_blob_storage=false`）。
  scan のための fetch は一時的で、scan 後に破棄される。
- URL submission を将来使う場合、media を配信する domain を Project Arachnid の設定画面で
  **Authorized Domains** に登録する必要がある（DNS / domain 所有確認を伴う）。media bytes 直送
  （既定）では不要。

## Shield から返るデータ（Match Data）の扱い

Shield の応答（Match Data）は kukuri 内部の critical safety decision の入力としてのみ使う。

- 実装は応答から `classification` / `match_type` **のみ**を読み取り、`near_match_details`・
  一致先 sha1/sha256・timestamp は型に存在しないため保存・転送・log 出力の経路が無い
  （`crates/cn-safety-arachnid`）。
- 公開・共有される moderation event には provider の生結果ではなく、kukuri 側の抽象化された
  判断（`exclude` / reason code 等）だけが載る。
- Match Data を P2P network / 他 operator へ流さない。LLM / AI moderation pipeline の入力や
  training / evaluation dataset にも使わない。

## `No Known Match` の意味

`no-known-match` は「**scan 時点で** Project Arachnid Shield の既知 Exact / Near Match に
該当しなかった」ことだけを意味する。

- **安全証明ではない**。未知の CSAM は known-match scan では検出できない。
- kukuri 内部でも `no_known_match` という reason code のまま扱い、`safe` / `clean` / `verified`
  相当の命名・表示に変換しない。

## 設定手順

1. Project Arachnid Shield account を取得し、API username / password を発行する。
2. credentials を env / secret manager に設定する（**repository / config ファイルに値を書かない**）:

   ```bash
   # .env（コミットしない）または Secret Manager からの注入
   PROJECT_ARACHNID_API_USERNAME=...   # 値は Project Arachnid 管理ページの表記に合わせた env 名
   PROJECT_ARACHNID_API_PASSWORD=...
   ```

   任意の上書き:

   ```bash
   PROJECT_ARACHNID_API_BASE_URL=https://shield.projectarachnid.com  # 既定値
   PROJECT_ARACHNID_API_TIMEOUT_SECS=30                              # 既定値
   ```

3. provider を known-CSAM slot に設定する:

   ```bash
   COMMUNITY_NODE_SAFETY_PROVIDER_KNOWN_CSAM=project-arachnid-shield
   COMMUNITY_NODE_SAFETY_PROVIDER_KNOWN_CSAM_REQUIRED=true
   ```

   `operator-config.yaml` では:

   ```yaml
   safety:
     profile: public-node
     providers:
       known_csam:
         provider: project-arachnid-shield   # underscore 表記も受理される
         required: true
         credential_secret_id: <secret-manager-id>
   ```

4. 検証する:

   ```bash
   # 実 API への connectivity check（credentials の値は表示されない）
   cargo run -p kukuri-cn-operator --features safety-arachnid -- \
     safety test-provider project-arachnid-shield

   # 静的 readiness check（public-node profile）
   cargo run -p kukuri-cn-operator -- safety readiness --config operator-config.yaml
   ```

## fail-closed 動作（public-node profile）

- credentials 未設定のまま provider を指定すると **runtime 構築が失敗**し、ingest は起動しない。
- Shield unavailable / timeout / 想定外応答の間、対象 content は index されない（`hold`）。
- Exact / Near Match は `exclude`（critical）として index / discovery / recommendation から除外される。
- `test` classification（C3P のテストデータ）は index されないが、`csam_confirmed` とは別の
  `provider_test_match` として記録される（統合検証で実運用イベントを汚さない）。

## credentials 漏洩時の手順

1. Project Arachnid の管理ページで該当 API credentials を無効化（rotate）する。
2. 新しい credentials を env / secret manager に再設定する。
3. node を再起動し、`safety test-provider project-arachnid-shield` で新 credentials を確認する。
4. 漏洩範囲（log / shell history / CI 変数等）を確認する。kukuri は credentials を log に出さないが、
   `.env` の共有・コミット等の運用ミスは operator 側で監査すること。

## 関連

- Issue #391 / #353、ADR 0027（`docs/adr/0027-deterministic-moderation-critical-safety.md`）
- `crates/cn-safety-arachnid`（provider 実装）
- `docs/runbooks/community-node-operator-docs.md`（operator-config / readiness 全般）
