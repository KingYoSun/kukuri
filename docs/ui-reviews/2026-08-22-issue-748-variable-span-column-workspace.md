# 2026-08-22 Issue #748 variable-span Column workspace Wave 0

- PR: [#749 Issue #748 Wave 0: 可変span Column Canvasのreview prototype](https://github.com/KingYoSun/kukuri/pull/749)
- Preview: [Column Canvas review preview](https://raw.githubusercontent.com/KingYoSun/kukuri/4d0825c3acf8735992bde6bf72626d55f2c02d46/docs/ui-reviews/assets/2026-08-22-issue-748/column-canvas-preview.png)
- Storybook review surface: `Review/VariableSpanColumnWorkspace/SingleTimeline`, `TimelineThreadProfile`, `ThreadChain`, `StreamTwoSpan`, `MetaverseThreeSpan`, `MetaverseFourSpan`, `MobileOneViewport`, `ControlCenterOpen`, `ColumnStates`, `ReducedMotion`
- Summary: ADR 0031 の Wave 0 review prototype として、可変 span の Column Canvas、Column 固有 footer action、左下の Control Center trigger、bottom drawer を既存 presentational component から構成した。production shell、store、route、protocol は変更していない。
- User flow summary: 保存済み layout がない初期状態は中央寄せの Timeline Column 1本、投稿 action、Control Center trigger だけを表示する。Timeline から Thread、Thread から Profile を右隣へ開き、各対象は同格の Column として title、scope、位置、span、pinned / temporary state を保持する。Stream は2 span、Metaverse は3 spanまたは4 spanの分割不能な surface とする。mobile はすべて1 viewportへ正規化し、位置 indicator、前後移動、Control Center の直接移動を併用する。
- Review result: desktop Column unit は `440px`、gap は `16px`、Stream は `896px`、Metaverse は `1352px` / `1808px` を採用した。desktop Control Center trigger は Column action と競合しない左下、drawer は下端から開く。mobile / desktop 境界は現行 responsive contract とそろえた `759px / 760px` とする。motion は `120ms / 200ms / 280ms`、移動量は Column `24px`、Control Center `32px`、reduced motion は `1ms / 0px` とする。
- Shneiderman 1 — consistency: Column kind 間で header、state label、scope、footer action、spacing、motion tokenを統一した。
- Shneiderman 2 — universal usability: drag grip、menu、primary action、Control Centerをkeyboardで到達可能にし、menuの左右移動をdrag代替にした。mobileにはswipe以外の前後移動と直接移動を用意し、主要targetを44px以上にした。
- Shneiderman 3 — informative feedback: active、pinned、temporary、dragging、drop targetをlabelと形状でも区別し、位置、総数、spanを非視覚情報として伝える契約を確認した。
- Shneiderman 4 — closure: Column固有footer action、Control Centerのopen / close、Column menuの操作境界を明示した。
- Shneiderman 5 — error prevention: 専用drag gripを使い、投稿、video、3D viewport、text selectionをdrag開始点にしない。複数span surfaceはatomicに扱い、scope / DraftをColumn単位で分離する。
- Shneiderman 6 — easy reversal: close、pin、左右移動を明示操作にし、parent relationを保持する。fullscreenは一時状態とし、終了後に元の順序、span、sessionへ戻る契約にした。
- Shneiderman 7 — internal locus of control: 利用者がColumnの追加、focus、pin、close、reorder、spanを明示的に選べる。初期状態から高密度layoutを強制しない。
- Shneiderman 8 — reduced memory load: title、scope、state、位置、spanをColumn上へ表示し、Control CenterのColumn一覧から任意の対象へ戻れる形にした。
- Viewport review: mobile `375 / 390 / 430px` は各Columnがviewportと同幅、desktop `760 / 1024 / 1280 / 1440 / 1920px` は440px unitを維持しながらCanvas内部だけが横scrollすることを確認した。全widthでdocument-level horizontal / vertical overflowは0だった。1440pxのStreamは896px、Metaverseは1352px、1920pxのMetaverse focusedは1808pxで表示した。
- Accessibility / motion review: dark / light、full / reduced motionを確認した。representativeなdesktop、mobile、Control Center openのStorybook accessibility scanはviolations 0、inconclusive 0。keyboard tabでskip link、drag grip、menu、primary action、Control Centerへ到達し、reduced motion時のcomputed tokenはduration `1ms`、distance `0px`だった。
- Exceptions: Wave 0 は設計判断とreview prototypeの確定までであり、実際のdrag / swipe / persistenceやproduction shell移行はWave 1以降で実装する。Figmaは使用していない。PRからimmutableに参照できるreview artifactとして、選定したpreview image 1枚だけをrepositoryへ置いた。
- Validation: `cargo xtask check`、`cargo xtask test`（Rust 584、harness 18、frontend 728）、`cargo xtask desktop-ui-check`（browser 14、visual 14）、対象prototype Vitest 4件、CSS token Vitest、desktop typecheck / lint、Storybook build、`cargo xtask oversized-files`、`git diff --check` がpass。oversized-filesは今回の追加ではなく既存のtracked file 3件だけをwarningとして報告した。
