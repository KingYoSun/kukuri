# Issue #920: 通知取得・state反映の責務集約

- Scope revision `2026-09-08-872-F-1R-v1`、区分C。before `0410261bb8091829db6b5cd8f1feb66791aaf327`。
- 先行#919はPR#929でmerge済み。全CI・独立監査PASS、merge tree一致を確認して開始した。
- 単一意図: badgeとinboxの異なる挙動を維持して、通知取得と結果setterの所有を一つのhookへ移す。

| 作業 | AC / INVAR | 対象・証拠 | 依存 |
| --- | --- | --- | --- |
| T1 変更前contract | AC-3、全INVAR | #919の固定INV1〜7/TR1〜8と29 targeted testsをbeforeでPASS | #919 |
| T2 取得owner抽出 | AC-1/2/4、INVAR-1〜5 | `useNotificationLoaders.ts`、data facade/effects/section loaderの最小配線。2 test fixtureのcompositionだけ同期 | T1 |
| T3 変更後検証・構造証拠 | AC-3/4/5 | 同29 tests、typecheck、desktop-ui-check、caller/owner比較、既存assertion不変 | T2 |
| T4 独立監査・CI・merge | AC-5、全INVAR | 固定headと別担当監査、必須CI、merge tree一致。結果はPR/Issue | T3 |

## Before / afterと全caller

| 指標 | before | after |
| --- | ---: | ---: |
| status/list取得と結果反映の製品module owner | effects + sectionの2 | notification loaderの1 |
| effectsのstatus/list API直接呼出 | 2 | 0 |
| sectionの通知inline実装 | 1 | 0 |
| mark-allの製品呼出site | 1 | 1 |
| 新loaderの製品生成site | 0 | facadeの1 |

`useDesktopShellData`が一度生成し、同じinbox callbackをsection loaderとeffectsへ、badge callbackをeffectsへ渡す。section単体testも本番と同じloaderを注入する。fallbackや二つ目のownerは置かない。pageのmanual refreshとfacade戻り値は不変。
元の2 callback本体を抽出し、badgeの`activePrimarySection`参照を引数に変えただけで、visibility/60秒周期/section変更時の即時refresh/markAsRead省略時true/Date.now/read_at投影/部分失敗とpanel errorの違いを維持する。
effectはtimer/event/section triggerとcleanupを引き続き所有する。private/adult payloadを保持し、OS notification dispatch、routing/store設計、Rust/IPC/schema/UI/locale/CSSは変更しない。

CodeGraphでsourceとcallerを確認し、次で全入口・sinkを補完した。

```powershell
rg -n 'getNotificationStatus|listNotifications|markAllNotificationsRead|refreshNotificationStatus|loadNotificationsSection|useNotificationLoaders' apps/desktop/src
```

本番のdirect status/list/markは新moduleのみ。登録点はdata facade→effects→runtime event/interval、active/background section、loadShellSections、page manual refresh。SQLite既読sinkは既存IPC/runtime/AppServiceのまま。INV/TRの増減0、対象未分類0。

## Validation

- before: #919新規contractと既存data/section/event bridgeの4 files/29 tests PASS、4.55秒。
- after: 同29 tests PASS、4.35秒。既存assertion変更0。変更した2 test fixtureは新loaderの生成・callback注入だけ。
- typecheck PASS。必須 `cargo xtask desktop-ui-check` は対象rootの実行directoryを確認して実行し、最終結果/CI/headはPR/Issueへ記録する。
- 最初のUI gateは共有build cache内のxtaskが別worktreeのpathを埋め込んでおり、そちらでeslint未検出となったためFAIL。製品の失敗として扱わず、xtask packageのbuild artifactを再作成してrootの正しいpathで再実行する。失敗runを成功証拠に数えない。
- Windowsのvisualはsmoke、Linux snapshot比較はCI。残る並行要求の完了順や取消モデルは変更しない。

## Rollback

この抽出PRだけをrevertし、#919のcontractを保持する。public API、schema、利用者データ変換は無い。
