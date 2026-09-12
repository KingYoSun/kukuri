# プロフィールの再取得と表示位置の安定化

## 作業範囲

- リスク区分B、Scope revision 2、基準commit `c0d3cefc67c7150bee961d300918748aa8fcc5c5`。
- 2026-09-12に承認されたプランの実装。自分のプロフィールカラムの選択時のちらつき、手動更新と回転表示、表示名・ユーザー名の2段表示、概要ヘッダー下の追加4pxを対象とする。
- ユーザーは実装・コミット・PR・必須CI成功後のマージを承認済み。
- Scope revision 2: 実装中の追加指示により余白を2pxから4pxへ変更し、狭幅・zoomでもアバターと名前のまとまりと編集ボタンを同じ行に保つ。759px以下で`flex-direction: column`にしていた既存規則を削除する。Windows 200%の行方向testで変更前の`column`を失敗として確認し、変更後の`row`で成功した。
- 他ユーザーの詳細プロフィール、backend / IPC / persist形式、公開投稿数の集計範囲は対象外。
- 製品契約は[DESIGN.md](../../DESIGN.md)、手順は[ADR 0014](../adr/0014-uiux-dev-flow.md)と[Issue lifecycle](../runbooks/issue-lifecycle.md)。

## 修正前の再現・変更後の振る舞い

ユーザー録画（Windows、1996×1356、30fps）の0.933秒で「プロフィールを読み込み中…」が挿入され、フォロー数・自己紹介・投稿を押し下げていた。`DataEffects`が選択sectionの変更で取得を行い、集約`loadShellSections`も無条件でプロフィールを取得していた。

修正前の再現testではカラム3回の選択で`listProfileTimeline`が6回増加し、確定済み0件の再取得が`ready`から`loading`へ戻ることを確認した。集約loader単体でも余分なreadが1回発生した。更新ボタンの受入testはボタン未実装で失敗した（合計4失敗 / 12成功）。

修正後は選択による取得増分0。初回・再open・必要な変更・手動更新を契機に取得し、確定済みデータは0件でも維持する。取得中の表示は固定寸法のヘッダーアイコンの回転とbusy状態へ移した。概要の冗長な見出しを表示名へ置き換え、ユーザー名を独立した下段に表示する。

## 取得経路のinventory

`codegraph node`によるsourceとcaller確認後、`loadProfileSection` / `loadShellSections` / `profilePanelState` / `loadTopics`の参照を検索し、取得とstate反映の入口を分類した。6 group、未分類0。新しいgroupは追加せず、既存の選択起点の取得を除去して明示更新入口を同じloaderへ接続した。

| ID | 入口 / member | 変更後の読み書き・責務 |
| --- | --- | --- |
| INV-1 | `useDesktopShellDataEffects`のown-profile effect、`loadShellSections`の初期化 | own-profileの存在変化で取得。集約loaderは未取得・未処理・初期loadingの時だけ初期化を補助する。成功済みやerror状態の選択だけでは再取得しない |
| INV-2 | `DesktopShellColumnWorkspace.activate`、route投影、live/game/bookmarks/settingsの`loadTopics` | 選択変更はプロフィールの有効値を無効化しない。他section自体の取得動作は維持 |
| INV-3 | ヘッダー更新、`DesktopShellPrimaryWorkspace`のRetry | 同じ`loadProfileSection`へ委譲。ヘッダーは処理中・保存中の再実行を抑止し、非選択の更新で選択を奪わない |
| INV-4 | `refreshVisibleTimelineAfterPublish`（`createOptimisticPost`の公開投稿・リポスト成功）、`handleSaveProfile`、`handleWithdrawPost`、`handleRelationshipAction`、`handleMuteAction`、`handleBlockAction` | 非選択のプロフィールも更新。後5種は集約取得の副作用に依存せず明示的にprofile loaderを呼ぶ。reactionの`setProfileTimeline`による確定値patchは維持。topic/channel設定・live/game等の操作はプロフィールを再取得しない |
| INV-5 | section loaderの成功 / 失敗 / finally、起動時の`getMyProfile`、保存開始 / 成功 | request IDは最新の要求だけを反映・busy解除する。保存revisionを跨ぐ古い応答を破棄する。初期bootstrapも新しい保存・取得結果を上書きしない |
| INV-6 | `ProfileOverviewPanel`、編集 / つながり画面、`resetProfileDraft` | `profileHasLoaded`は概要の全取得成功でのみ立てる。保存・draftリセットによる`ready`とは独立。`profileDirty`、画像preview、modeを保持する |

`profileSaveRevision`と取得履歴・busyはsession内stateのみ。既存の権限・通信・永続化guardを変更しないため、区分Cへの変更や独立監査の追加条件はない。

## 受入条件・状態遷移と証跡

| 条件 | 実装・検証 |
| --- | --- |
| AC-1: 選択のみで取得せず位置を維持（TR-3） | `DesktopShellPage.profileRefresh.test.tsx`の選択往復、`useDesktopShellSectionLoaders.test.tsx`の集約loader、`profile-refresh-layout.spec.ts`のブラウザー操作 |
| AC-2: 初回 / 再open / 必要な変更 / 再試行（TR-1 / TR-2 / TR-6） | profileRefreshの再open test、既存`DesktopShellPage.profile.test.tsx`・socialGraph・`profile-post-refresh.spec.ts`の非選択への投稿反映 |
| AC-3: 更新回転・停止・重複抑止（TR-2 / TR-7） | profileRefreshの連打・Enter・focus保持、browserのbusy・animation-name・取得回数、Windows WebView確認 |
| AC-4: 0件も取得済み表示を維持（TR-2 / TR-5） | loaderの空プロフィール更新・error回復、profileRefreshの空feed保持、browserの応答保留中4frameの概要・投稿・body bounding box比較 |
| AC-5: 初回error / 更新error / 旧応答 / 保存競合（TR-4 / TR-5 / TR-7） | 既存初回error→Retry、loaderの旧成功・失敗・旧finally、保存revisionを跨ぐ応答、最新値と未保存draft保持 |
| AC-6: 名前を2段に表示（TR-8） | `ProfileOverviewPanel.test.tsx`、localizedAccessibleNames、Overview stories、browserの名前と長文overflow確認 |
| AC-7: 概要ヘッダー下に追加4px（TR-8） | 局所padding。browserとWindows WebViewのcomputed style確認。avatarの既存3rem・全丸規則は維持 |
| AC-8: locale / theme / 入力 / motion（TR-8） | browserのja-dark/light 1280px、en-dark/light 1024px、zh-CN-dark/light 390px、OSとreview属性のreduced motion。Windowsの通常・200% zoom |
| INVAR-1: draft / focus / scroll / route | 既存draft保持test、保存競合test、ヘッダー更新時の選択不変・focus保持、browserの位置不変 |
| INVAR-2: 公開済み投稿の非選択への反映 | 既存profile・公開/非公開投稿の回帰testを維持。未送信draft・集計範囲の仕様変更なし |
| INVAR-3: 他の面・API / persist | 共有型変更なし。通知等の既存ヘッダー操作を維持しfrontend全体gateで確認 |

## 検証結果

- 修正前: section loader + profileRefreshの再現test、4失敗 / 12成功。失敗理由は上記の通り。
- 修正後targeted Vitest: section loader + profileRefresh + profile + socialGraph、30成功。その後追加の旧finally・保存競合・欠落usernameを含むloader / Overview / accessible names / actionsの4ファイル、42成功。
- Playwright `profile-refresh-layout.spec.ts --project=chromium --workers=2`: 3幅×投稿あり/0件の6成功。応答保留中の位置、busy、回転、reduced motion、失敗と再試行を確認。長文ケース追加後の結果は全体gateで確認する。
- `cargo xtask oversized-files`: 成功。既存baselineの増加なし。
- `cargo xtask desktop-ui-check`: 初回実行成功（182ファイル / 1520 tests、Storybook、browser、visual smoke）。Scope revision 2のCSS変更後に全gateを再実行中。最終結果とLinux視覚baseline・CI結果は完了時に追記する。

### Windows描画確認の範囲

同じfrontend sourceをViteで配信し、既存Tauri review hostのWindows WebView2（Edge 152.0.4191.66）へ読み込んだ。backendはdesktop mockで、実アカウントのデータ取得・通信は検証対象にしていない。

- 通常: 1280×800 CSS px、DPR 1.5。選択往復のprofile read増分0、手動更新1回、連打/Enterの重複抑止、更新前・保留中・完了後の概要/投稿位置一致、回転、追加4pxを確認。
- native `set_zoom(2)`による200%: 640×400 CSS px、DPR 3。同じ操作と位置比較が成功。CDPのスクリーンショットは拡大時に一部を切り取るため、画面全体の画像証拠とは扱わない。
- 証跡はsession内`.codex/plans/profile-native/`のJSON・画像。元録画のアプリ/WebView versionは不明なため、完全に同一versionの実機比較とは区別する。
- screen readerによる読み上げは未実施。accessible name / busy / keyboard focusを自動検証し、読み上げ適合を断定しない。

## 完了時の更新

必須frontend gate、Linux視覚baseline、PR headのCI結果を記録し、CI成功後にマージする。コミット・PR・マージは承認済みで、追加の確認待ちは設けない。
