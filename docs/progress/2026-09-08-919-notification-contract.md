# Issue #919: 通知取得モードの変更前contract

## 範囲と開始条件

- Issue: #919、Scope revision `2026-09-08-872-F-1C-v1`、リスクC。
- 変更前commit: `ac9946d66aaf197204f669a10701fac8a13877e9`。後続#920の製品変更より前に実行する。
- ユーザーは#919〜#928について、各Issueの必須CI・独立監査成功後のmergeまで承認済み。#928にも依頼どおり独立監査を行う。
- 本Issueの製品コード、IPC、DB、既存test、UI描画変更は0。新規testと本記録のみ。
- 正本はIssueのAC-1〜4、INVAR-1/2、INV-1〜7、TR-1〜8、ADR 0023、DESIGN、ADR 0014。未定義のcancel/retryは追加しない。

## 作業と受入条件

| 作業 | AC / INVAR | 対象・証拠 | 依存 |
| --- | --- | --- | --- |
| T1 既存保護と入口確認 | AC-1、INVAR-1 | data hook/section loader/event bridge、全通知API callerと動的登録、既存16 testsのbaseline | なし |
| T2 不足する禁止I/O・state contract | AC-2/3、INVAR-1/2 | `apps/desktop/src/shell/data/notificationLoaders.contract.test.tsx`。TR別対応は下表 | T1 |
| T3 変更前製品で検証・記録 | AC-4、INVAR-1/2 | targeted + `cargo xtask desktop-ui-check`。source diffと既存assertion不変 | T2 |
| T4 独立監査・CI・merge | AC-1〜4、INVAR-1/2 | 固定PR headの別担当監査、CI、merge tree照合。結果はIssue/PRで保持 | T3 |

## Inventoryとcontract対応

製品inventory変更は0。CodeGraphでuseDesktopShellData/effects/section loaders/event bridgeを読み、macro/event登録と次の全caller検索を補完する。

```powershell
rg -n 'getNotificationStatus|listNotifications|markNotificationRead|markAllNotificationsRead|loadNotificationsSection|refreshNotificationStatus' apps/desktop/src
rg -n 'mark_all_notifications_read|notification_status_changed' apps/desktop/src-tauri crates/desktop-runtime crates/app-api
```

| INV / TR | 今回追加する観測 / 既存証拠 |
| --- | --- |
| INV-1/2、TR-1 | `hidden mount, interval and runtime event issue no notification requests`。status/list/markすべて0、state保持 |
| INV-1/2、TR-2 | `outside inbox, badge count ...`と`active inbox badge count ...`。inside/outside × count0/1、list条件・mark0、既存payload保持 |
| INV-1/2、TR-3 | `badge ... failure keeps previous rows ...`。status failure/list failureを区別、部分成功のstatusだけ反映、panel不変 |
| INV-3/4/6、TR-4/7 | `... inbox read marks only unread rows ...`。defaultとbatch入口、mixed read_at/payload、既読時の追加mutation0。既存data hookのloadTopics testも維持 |
| INV-3/5、TR-5 | `background refresh preserves mixed private/adult/read payloads ...`。list/unread/read_at保持・mark0。page manual refreshとbackground columnは既存notifications integration |
| INV-3/6、TR-6 | `inbox ... failure keeps both previous results ...`、`read failure keeps unread data ...`。取得失敗とmark失敗で保持state/errorの違い、mock保存値も確認 |
| INV-1/2、TR-8 | `active inbox badge count ... resets timer and event callback`。section変更の即時refresh、旧interval解除、新60秒周期、最新event callback、unmount/remountでlistener解除 |
| INV-7、TR-7 | 新規mixed fixtureのprivate channel/DM/adult labels/順序保持。表示・クリック・OS activation・focusは既存notifications/notificationFocus/osNotificationActivation/adultContentGating integration |

テストはpublic data facadeとsection loaderを使い、effects内部helperをmockしない。badge測定では背景inbox Columnだけをfixtureから除き、独立したsection取得と混同しない。inboxの永続read処理は既存AppService contractが所有し、本Issueはfrontend API呼出とstate/mock保存値を観測する。

## 実行記録

- before: `npx pnpm@10.16.1 --dir apps/desktop test src/shell/useDesktopShellData.test.tsx src/shell/data/loaders/useDesktopShellSectionLoaders.test.tsx src/shell/data/useRuntimeEventBridge.test.ts` → 3 files / 16 tests PASS、3.67秒。
- 初回の新規testで、mockが補完する`actor_picture_asset: null`をfixtureに持たせていない差を検出。既存mockの入力正規化に合わせてfixtureを明示し、製品変更は行っていない。
- targeted: 同commandに新規notificationLoaders.contract.test.tsxを加えて4 files / 29 tests PASS、4.14秒。全frontend gate結果とPR headは実行後に追記する。Linuxのvisual比較はCIで確認し、Windows smokeを比較PASSと扱わない。

## 差し戻し・残存条件

test-only PRを単独revert可能。#920に着手後は保護網だけを外さず先に#920を戻す。
並行要求の完了順やunmount時の新しい進行中要求取消しは対象外。CIや独立監査未完の段階を完了扱いしない。
