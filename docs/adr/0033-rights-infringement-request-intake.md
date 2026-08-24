# ADR 0033: 権利侵害申出の受付境界と Community Node の対応範囲

## Status

Accepted

## Date

2026-08-25

## Context

Community Node は、自身が提供する索引、検索、発見、推薦、moderation、blob cache には node-local な送信防止を適用できる。一方、投稿の正本、第三者端末、他の Community Node、Direct P2P の通信、暗号化された relay packet を一律に削除・遮断する authority は持たない。

一般通報は spam や safety 上の問題を知らせる入口であり、権利者または代理人が権利、対象、根拠、証拠参照、連絡先を示して継続的な処理状態を確認する権利侵害申出とは、必要な情報、本人性、保存期間、operator workflow が異なる。両者を同じ record に保存すると、申出人に実現不能な救済を期待させ、内部の safety 判定と法的判断も混同する。

## Feature Data Classification

- Feature 名: Community Node への権利侵害申出
- Durable / Transient: Durable
- Canonical Source: 当該 node の Postgres にある申出 record と append-only event
- Replicated?: No
- Rebuildable From: 再構築不能。申出 record と event を backup 対象にする
- Public Replica / Private Replica / Local Only: Local Only
- Gossip Hint 必要有無: 無
- Blob 必要有無: 無。証拠ファイルは受け取らず URL、hash、識別子だけを保存する
- SQLite projection 必要有無: 無
- 必須 contract: scope revision 同意、server-side scope 判定、追跡 secret、状態遷移、送信防止との同一 transaction、PII 非公開
- 必須 scenario: 申出人が node の可能・不可能な対応を確認してから申請し、operator の措置後に追跡画面で公開可能な状態だけを確認できる

## Decision

### 1. 一般通報とは独立した opt-in capability とする

権利侵害申出は `rights_request_endpoint` capability を有効にした node だけが提供する。manifest は専用の申出 URL、説明文書 URL、初回応答目標日数を optional field として公開する。一般通報の `/v1/report` は権利侵害申出を受け付けず、対応 node の専用 URL を案内する。

申出 schema、保存 table、operator workflow、audit event は一般通報・異議申立てと分離する。関連する送信防止 record には申出 ID を関連 record ID として接続できるが、理由や申出人情報を複製しない。

### 2. 申請前に版付きの対応範囲を明示し、同意を必須にする

公開 scope endpoint と申出画面は、少なくとも次を申請フォームより前に示す。

- 当該 node で現在有効な capability と、対象を確認できた場合に取り得る node-local な措置
- 他 node、第三者端末、投稿正本、Direct P2P、暗号化 relay packet、既取得データには強制力が及ばないこと
- 申出受付は権利侵害の認定や requested action の保証ではないこと
- 初回応答の運用目標と、法定期限を表すものではないこと
- 申出情報を operator が審査し、必要に応じて送信者へ連絡する場合があること

scope 文面、capability、authority scope、応答目標から安定した `scope_revision` を生成する。申出 API は `scope_acknowledged=true` と現在の `scope_revision` を必須にし、欠落または旧版なら `409` で最新 scope の再確認を求める。表示だけ、pre-checked checkbox、client 側判定だけでは受付しない。

### 3. 対応可能性は server が独立に判定する

client が指定した capability は希望として扱う。server は manifest 上の有効 capability と node-local な対象記録を照合し、申出全体を次のいずれかに分類する。

- `verified_scope`: 対象と要求 capability を node が確認でき、node-local な措置候補がある
- `unverified_scope`: 形式上は対応候補だが、対象または根拠を現時点で確認できない
- `out_of_scope`: node が提供していない capability、未対応の対象、または authority 外の救済だけを求めている

`verified_scope` の初期状態は `received`、`unverified_scope` は `needs_information`、`out_of_scope` は `out_of_scope` とする。client 申告だけで `received` にしない。

### 4. 最小限の申出情報と accountless な追跡手段を持つ

申出には、申出人区分、氏名・連絡先、代理権の根拠、権利カテゴリと権利根拠、対象識別子、侵害態様、許諾していない旨、requested capability、証拠参照を保持する。証拠参照は URL、hash、外部識別子に限定し、ファイル upload や対象コンテンツの複製は行わない。

受付時に公開用 reference ID と一度だけ表示する tracking secret を発行し、server は secret の hash だけを保存する。公開 status／withdraw endpoint は reference ID と secret の組を要求し、申出人 PII、operator、内部メモ、非公開判断理由を返さない。存在しない ID と secret 不一致は同じ応答にする。

### 5. 状態遷移と措置を監査可能にする

状態は `received`、`needs_information`、`reviewing`、`sender_contacting`、`actioned`、`declined`、`out_of_scope`、`withdrawn` とする。各遷移は actor、時刻、公開可能な説明、外部通知の記録を append-only event に残し、operator audit に申出内容を含まない要約を同じ transaction で追記する。

`actioned` への遷移が Community Node 内の送信防止を伴う場合、送信防止 record、申出状態、申出 event、operator audit を同じ transaction で確定する。外部通知は自動 SMTP を前提にせず、公開 status と operator が記録する delivery status を正とする。

## Consequences

- 申出人は送信前に、その node が実行できる措置と authority 外の領域を明示的に確認する。
- node ごとに capability が異なっても、server-side scope 判定により受付状態が過大表示されない。
- PII と権利主張は public replica や一般通報へ流れず、公開追跡面も必要最小限になる。
- scope 文面や capability の変更後は、古い画面を開いたままの申請を再確認させられる。
- operator は法的判断と safety 判定を分離したまま、既存の node-local 送信防止へ接続できる。

## Out of Scope

- 権利侵害の自動認定または法的助言
- 他の Community Node、第三者端末、source peer、author-owned replica の遠隔削除
- Direct P2P の遮断、暗号化 relay packet の内容検査、既取得データの回収
- 証拠ファイルの upload、保管、malware scan
- 自動 SMTP 配信、郵送、裁判所・行政機関への自動提出
