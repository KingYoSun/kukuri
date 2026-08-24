# 2026-08-23 Issue #748 production Columns Wave 3

- Preview: [scope別 Timeline と Column footer Composer](assets/2026-08-23-issue-748-wave-3/scoped-column-composer-production.png)
- Storybook review surface: `Review/ProductionColumnWorkspace/ScopedDraftsAndComposer`。`demo / Public`、`demo / Core friends`、`iroh / Public`を同時に置き、private Columnだけに展開済みDraftを保持する。
- Summary: Timeline、Thread、Conversationのprimary actionを各Column footerへ移し、上向きに展開するComposerをproduction surfaceへ接続した。Streamはstart / join / leave / end、Gameはcreateの既存domain actionだけをfooterへ表示する。
- Scope ownership: Timeline identity、load / refresh / pagination / pending cache、Live / Game一覧は`ColumnScope`のtopic + channelを正本にする。publicの`null` channelをglobal private selectionで上書きせず、public / private / 別topicの本文を同時にDOMへ保持する。
- Destination clarity: headerとComposerに利用者向けchannel label + topic末尾、Thread、peer display nameを表示する。raw thread ID / peer pubkeyだけを主labelにしない。
- Draft protection: Draft keyはcolumn、post / reply / message、topic、channel、thread / peerを含む。focus、scope navigation、別Draftの送信、Column closeでは移動・消去せず、成功したDraftだけをclearする。失敗時は対象Draftの本文・media・errorを残す。
- Attachment lifecycle: preview URLはDraft itemごとに登録・解放し、対象Draftのremoveまたは送信成功時だけreleaseする。別Columnのattachmentは保持する。
- Active / inactive action: active Columnはicon + label、inactive Columnはaccessible nameを持つ40×40px icon buttonにする。ComposerのCloseと送信は同じfooter内に留める。
- Narrow / overflow review: in-app browserで759、760、1024、1440pxを確認した。document-level overflowは全幅で0px、Canvasだけが横scrollを所有する。初期routeのactive Columnもmount時に`scrollIntoView`し、1440pxで左右端がviewport内に入るよう修正した。
- Keyboard / accessibility: Columnはregionのaccessible nameにkind、位置、span、active、pin stateを含む。footer action、Close、attachment、submitはnative controlで操作し、inactive icon actionにも`Publish to …` / `Message to …`等のdestination込みaccessible nameを付ける。
- Shneiderman 1 — consistency: post / reply / messageは同じfooter展開、destination、attachment、error、submit構造を使う。
- Shneiderman 2 — universal usability: labelを隠すinactive状態でも40px targetとaccessible nameを維持し、textarea、file input、buttonの標準keyboard操作を使う。
- Shneiderman 3 — informative feedback: active border、expanded Composer、pending disable、Draft error、optimistic post stateを対象Column内に返す。
- Shneiderman 4 — closure: API成功後に対象Draftだけをclearし、optimistic itemを`syncing`へ進め、scope固有timelineをrefreshする。
- Shneiderman 5 — error prevention: destinationを常時表示し、public / privateの`null`判定を明示し、別scopeのglobal projectionを送信引数へ流用しない。
- Shneiderman 6 — easy reversal: ComposerはCloseで折りたため、Column close後も同じidentityを再度開けばセッション中Draftを復元する。
- Shneiderman 7 — internal locus of control: footer actionは自身のColumnをactiveにしたうえで自身のDraft keyを開き、背景Columnのstateを変更しない。
- Shneiderman 8 — reduced memory load: headerとComposerの両方に送信先を置き、Thread replyは対象post本文もbannerへ短く再表示する。
- Review result: 初回reviewで初期active ColumnがCanvas外へ一部隠れる問題を検出し、initial mountでもactive Columnへscrollするよう修正した。3つのscope Columnとprivate Draftが同時に保持され、document overflowが発生しないことを確認した。
- Linux baseline: PR初回CIのvisual artifactを確認し、今回意図したColumn footer、active Columnへの初期scroll、Stream actionが反映された6画面のactualを正規baselineへ更新した。
- Exceptions: Control Center、sidebar撤去、可変span / reorder / layout persistence、restart後Draft復元、mobile paging、immersive lifecycleはWave 4〜6へ残す。Metaverseの新規chat / comment contractは追加していない。
- Validation: production Storybook実画面reviewに加え、`cargo xtask check`、`cargo xtask test`（Rust workspace 584件、harness 18件、frontend 747件）、`cargo xtask desktop-ui-check`（lint、typecheck、Vitest 89 files / 747 tests、Storybook build、Chromium 16件、visual regression 14件）、`cargo xtask oversized-files`、`git diff --check`をローカルで完了した。Linuxを含むリポジトリCIは下記PRの結果を一次記録とする。


## 追記（2026-08-24、Issue #768）

- PR: https://github.com/KingYoSun/kukuri/pull/755 （本文中の「下記PR」はこの PR を指す）
- Reduced motion: Wave 3 は新規 transition / animation を追加しておらず、Composer の展開は Wave 0 で確定した motion token の範囲内で動作する。`prefers-reduced-motion: reduce` 時に token が 1ms / 0px へ縮退することを Storybook toolbar の Reduced motion で確認した（Wave 0 record の確認手順に同じ）。
