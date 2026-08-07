# 2026-08-06 — #617 capability の「提供中」昇格と届出用構成図・運営者開示の実態同期

参照: [Issue #617](https://github.com/KingYoSun/kukuri/issues/617)（親: #612。前提の #616 は完了済み）/
計画: `.claude/plans/2026-08-06-issue-617-capability-disclosure-sync.md` /
PR: [#640](https://github.com/KingYoSun/kukuri/pull/640)（T1 昇格）·
[#641](https://github.com/KingYoSun/kukuri/pull/641)（T2-T6 開示同期）

## 到達点

`CommunityIndex` / `Moderation` / `CommunityLocalTrust` を「計画中（この配布物では未提供）」から
「提供中」へ昇格し、operator-config を単一の入力元として、届出の再提出に転記できる
構成図・役務説明・外部送信・保持方針の一式を実態どおり生成できるようにした。

## 変更の要点

- **昇格（T1）**: 3 capability の availability を Available へ。承認フラグ
  （`acknowledge_planned_capabilities`）無しで有効化でき、設定されていても後方互換で受理する。
  manifest の `capability_scope` は available 側へ 3 capability が入り `planned_enabled` は
  空になる（wire 契約はスキーマ不変・値のみの変化。golden と desktop-runtime round-trip を更新）。
  features 無効の node で「提供中」と誤表示されない契約は維持。
- **説明の実態化（T2）**: 「（計画）」表記を実装済みのデータフロー説明へ更新
  （許可 content のみ索引・fail-closed・Match Data 非保存・node-local advisory・opt-out 可逆など）。
- **外部送信の動的開示（T3）**: `SafetyProviderEntry.hosting`（self_host / external。未指定は
  保守側 = 第三者への外部送信）を導入し、外部送信表示へ「安全性走査プロバイダへの送信」節を
  プロバイダ構成（真実源）から動的生成。Project Arachnid Shield は第三者への外部送信
  （メディアのバイト列またはハッシュ）、視覚言語モデルは hosting 区分で
  「運営者管理基盤内の送信」と「第三者への外部送信」を切替。接続先 URL・内部アドレス・
  資格情報は生成物に出さない（非含有監査テストで固定）。
- **保存・保持（T4）**: データ保持ポリシーへ「データ区分と保存先」表と削除・再構築・
  バックアップの節を追加（Postgres = 永続・真実源 / ArcadeDB = 再構築可能・バックアップ対象外 /
  Valkey = TTL 揮発 / 生メディア = 恒久保存しない / indexer 同期状態 = canonical ではない）。
- **構成図・役務説明（T5）**: 構成図へ「構成要素とデータフロー」（利用者端末・他ピア /
  ノード内部 / 外部・運営者基盤の 3 ブロック）と境界の要点（公開面の限定・authority の
  supported scope 限定・保存区分・暗号化済み traffic の中継があり得ること）を追加。
  届出補助資料へ「提供するサービス: P2P コミュニケーションネットワークの補助サービス」
  「使用するサーバー: <cloud_provider>」の転記用行を追加。
- **全文書の同期（T6）**: moderation-policy の「未提供」分岐を走査の流れ（fail-closed・
  既知一致 + 分類器・Match Data / 生応答の非保存）と申し立て導線の説明へ置換。
  昇格後の全生成物に「計画中」表記が残らないことを監査テストで固定。

## 実 config からの生成・監査（T7）

- 実運用 operator-config.yaml（gitignored。`hosting: self_host` を追記）から
  `validate-config` / `generate-docs` / `check-disclosures` を実行し、12 文書の生成と
  drift なしを確認した。
- 非含有監査: 生成物に VLM の接続先ドメイン・IP アドレス・経路情報（WireGuard 等）・
  image digest・資格情報・「計画中」表記が含まれないことを grep で確認した。
- 届出転記用の一式（network-diagram.md / telecom-notification-draft.md /
  external-transmission-notice.md / data-retention-policy.md）を運営者へ引き渡した。
  様式第 1〜3 への転記・提出は運営者の手作業（生成物は実態の全外部要素を含み、
  様式に載せる粒度は運営者が選ぶ）。

## 残作業・持ち越し

- public manifest が指す `GET /terms` / `GET /privacy` / `GET /external-transmission` /
  `GET /moderation-policy` / `GET /abuse-policy` は、operator config から生成した同一の開示文書を
  `cn-user-api` が Markdown として配信する。operator config 未設定時は manifest と同様に 404。
- 実機（GCP VM）の public manifest と開示 URL への反映は、新 image の digest 更新 + apply が必要。
  現在は届出受領前の拘束条件により DNS 閉鎖中のため、再公開時（届出受領後）に
  image 更新とあわせて行う（`docs/progress/2026-08-06-616-activation.md` の再公開手順参照）。
- kukuri.app サイト側（#602）の公開物更新は再公開時に接続する。
