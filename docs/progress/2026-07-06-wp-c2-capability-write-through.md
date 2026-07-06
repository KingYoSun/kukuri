# 2026-07-06 WP-C2 private channel capability 永続化の write-through 集約(完了報告)

参照: `.claude/plans/2026-07-02-refactoring_master_plan.md` Phase 2 / WP-C2(finding D-3 / E-2)、
個別プラン `.claude/plans/2026-07-06-fix-c2-capability-write-through.md`、`REFACTORING.md` fix 規律・
boundary 規律・大型ファイルポリシー。PR: #476(export persist fix)/ #477(keyring stale-entry)/
#478(永続 JSON fixture)/ #479(boundary: write-through callback)/ #480(マージ事故 hotfix)。

## 問題(finding D-3 / E-2)

private channel capability registry(epoch 秘密を含む)の永続化義務が desktop-runtime 側の
「変異ラッパーが手動で persist を呼ぶ」規約(9 箇所)に漏出しており、export 系ラッパーには
persist が無かった。owner の export(共有)は `private_channel_state_for_owner_action(Share)`
経由の auto-rotate(InviteOnly / FriendPlus は毎回無条件)で in-memory capability を新 epoch に
置換するため、export → 再起動で新 epoch secret がサイレントに失われる。owner は rotation grant の
配布対象外(rotate 時にスキップ)で自動回復経路がなく、次の export で二重 rotate してメンバーと
恒久分断する(E-2)。さらに調査で、persist 漏れは export に限らず write 系経路(投稿・live・game・
metaverse room 作成での FriendOnly auto-rotate / 全経路の grant redeem)にも存在することを確認した。

## 実装した範囲

- **#476(fix)**: failing test 先行(restart 型 2 本 — InviteOnly 直呼び / FriendPlus ×
  access_token ディスパッチャ経由。再起動後に `current_epoch_id` が巻き戻る赤を実測)→
  export 系 **4 ラッパー**に persist を追加。マスタープランの「3 経路」は app-api 側 Share 変異点の
  数で、`export_channel_access_token` は app-api 内部で 3 export へ委譲するためラッパーとしては
  4 本目の独立修正が必要だった。
- **#477(fix、スコープ追加)**: Auto モードの `persist_optional_secret` は keyring set 失敗時に
  file へフォールバックするが stale keyring entry を削除せず、load(keyring 優先)が古い値を返し
  続ける既存バグを修正(set 失敗時に best effort で delete してから file へ)。Windows では
  keyring crate が `CRED_MAX_CREDENTIAL_BLOB_SIZE = 2560`(UTF-16)を書き込み前検証で決定論的に
  失敗させるため、registry JSON が 2 チャンネル分(実測 1850 bytes = UTF-16 3700)で発現する。
- **#478(contract)**: 永続 registry JSON(`Vec<PrivateChannelCapability>`、凍結境界)の形状
  golden + 最小/legacy 形の読み込み互換 + diagnostics 3 フィールドが復元結果に影響しないことの
  固定(これまでこの JSON を固定するテストは存在しなかった)。
- **#479(refactor:boundary)**: `AppService` に同期 callback
  (`PrivateChannelCapabilityPersist`)の OnceLock フィールド + setter を追加(コンストラクタ
  シグネチャ不変 = 約 80 caller 無変更)。registry の変異 2 点(`register_joined_private_channel`
  の insert 直後 / `remove_joined_private_channel` の remove 直後 — 全変異経路がこの 2 点に集約
  されることは grep 網羅 + 敵対的検証で確認)で「専用 guard 内: state-only スナップショット取得 →
  persist」を発火。desktop-runtime は復元ループ完了後に注入し、手動 persist 13 箇所と
  `persist_private_channel_capabilities_from_app` を削除。スナップショットは diagnostics
  (docs 読みを伴う async/fallible)に依存しない純関数で、diagnostics 3 フィールドは既定値で
  永続化(復元互換は #478 が固定。JSON 形状不変)。lost-update(従来は変異→snapshot→persist を
  貫く排他なし)も guard 直列化で解消。
  - `private_channels_support.rs` 1028 → 1078 行の oversized baseline 更新(正当化は PR 本文)。
- **#480(hotfix)**: #476(squash)→ #479(#476 ブランチ由来)の 3-way マージで、export 系
  4 ラッパーだけ main 側の手動 persist 呼び出しが復活し、削除済みヘルパを参照して main が
  ビルド不能になった。#479 の最終形へ復元。**教訓: stacked PR と squash マージの併用は、後続 PR の
  マージで共通祖先とのテキスト差分が交錯し壊れた自動マージを生みうる。stacked PR 利用時はマージ後の
  main のビルド確認を必須にする。**

## 「アトミック化」の操作的定義(受入)

厳密なトランザクション性は不可能(変異は in-memory HashMap、永続化は keyring/file の別ストア)
のため、次の弱い定義を採用した: (a) registry を変異させるメソッドは persist 試行完了まで return
しない、(b) 専用 guard 内でスナップショット取得 → persist を直列化(全量スナップショットのため
最後の persist が必ず最新を反映)、(c) rollback なし(persist 失敗はエラー伝播 — 現行同等)。

## 検証

- 各 PR で `cargo xtask rust-test` green(#479 は `e2e-smoke` も)。app-api 新規テスト 6 本
  (fixture 3 + counting callback 3)、desktop-runtime 新規 restart テスト 2 本。テスト削除ゼロ。
- **Windows 実機確認(Auto = 実 Credential Manager、GUI 非経由の検証 runner)**:
  - ケース A(keyring 内): 1 チャンネル + export 1 回 → fallback ファイル 0(keyring 保存)→
    再起動後も rotate 後 epoch 維持 + 旧 epoch archived。
  - ケース B(上限超過): 2 チャンネル + export ×3 → registry 1850 bytes で keyring set が実際に
    失敗 → file fallback 発生(1 ファイル)→ #477 の stale entry 削除が効き、再起動後に両チャンネル・
    全 rotate 履歴(archived 3 世代)を完全復元。#477 なしでは 2 個目のチャンネルがサイレント消失する
    経路そのもの。
  - タイムスタンプ付き記録は #479 のコメントに残した。

## 残課題(別 WP / 別 issue)

- `gossip_subscription_state` の同型手動 persist 規約(2 箇所)の callback 化(boundary の形の横展開)。
- persist ファイル書き込みの atomic 化(temp+rename+fsync なし)と破損 JSON での起動失敗
  (Failed 画面で DB エラーと誤誘導表示)— 既存の独立問題。
- keyring サイズ構造問題の根治(registry を keyring 外へ移す再設計。現状は閾値超過後、常時 file
  保存で機能維持)。
- H5-(4): persist 境界是正が完了したため、AppService 分割の前提が整った(マスタープランの依存
  C2 → H5-(4) を解消)。
