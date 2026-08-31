# 2026-08-30 Issue #820 Explore result PostCard

- Status: current
- Supersedes: None
- Superseded by: None
- PR: https://github.com/KingYoSun/kukuri/pull/842
- Previous PRs: https://github.com/KingYoSun/kukuri/pull/837, https://github.com/KingYoSun/kukuri/pull/839
- Preview: [Windows 11 / Tauri / dark](./2026-08-30-issue-820-explore-post-card-windows.png)
- Surface / user / purpose: Explore の検索・発見・おすすめ結果で、利用者が Timeline と同じ投稿Cardの視覚・著者導線・ID操作・通報導線を使って索引結果を確認する。
- Summary: Explore 固有の簡易結果Cardを共通 `PostCard` の読み取り専用表示へ置き換えた。索引レスポンスに存在する投稿ID、著者公開鍵、本文、時刻、公開範囲だけを表示し、不足するエンベロープ、添付、反応、返信、リポスト等は補完しない。preview注意書きと生のscope識別子は画面から削除した。
- Conditions:
  - Platform: Windows 11、Tauri development build、WebView2。実データ接続とrepositoryのdeterministic mockの両方を使用した。
  - Viewport: Windows実機は2560×1392の最大化desktop window。Playwrightは既存のdesktop / narrow設定を使用した。
  - Theme: Windows実機はdark。visual smokeではdark / lightを確認した。
  - Locale: Windows実機はja。component / browser testでは既存のen / ja / zh-CN翻訳資源を通した。
  - State: Exploreのeligible Community Nodeは`api.kukuri.app`。実データ接続では検索・発見が空、deterministic mockでは検索1件、発見4件、おすすめ4件を確認した。
- Accessibility / interaction: 結果は`list` / `listitem` / `article`として公開され、著者、公開範囲、時刻、本文、通報のaccessible nameを維持する。読み取り専用Cardには返信・リポスト・リアクション・ブックマークを出さない。Computer Useで検索・発見・おすすめをpointer操作し、操作切替直後に旧結果が消えること、右クリックmenuが投稿ID・著者IDだけを提示してエンベロープIDを捏造しないこと、通報dialogが検索では「検索結果」/ community index、おすすめでは「おすすめ」/ recommendationとして`api.kukuri.app`へrouteすることを確認した。
- Performance: 新規dependency、polling、API、永続状態は追加していない。既存レスポンスをpure adapterで`PostCardView`へ変換し、既存`PostCard`を再利用するため専用計測の対象外とした。
- Validation:
  - failing-first: 索引結果adapter、共通PostCard表示、読み取り専用通報、明示ID menu、操作切替時の結果失効を表す回帰テストが実装前に失敗することを確認した。
  - targeted Vitest: 52 tests passed。
  - targeted Playwright: Community Index 3 tests passed。
  - typecheck / lint passed。
  - `cargo xtask check` passed。
  - `cargo xtask test`: Rust 694 passed / 3 skipped、harness 22 passed、frontend 129 files / 959 tests passed。
  - `cargo xtask desktop-ui-check`: lint、typecheck、frontend 959 tests、Storybook build、browser Playwright 49 tests、visual smoke 14 tests passed。
  - Windows実機: 実データ接続で空状態を確認後、同じTauri / WebView2をdeterministic mockで起動し、検索・発見・おすすめ、旧結果失効、ID menu、検索結果 / おすすめ通報routing、Cardの切れ・横overflow・操作中のちらつきがないことをComputer Useで確認した。
- Not verified: 物理タッチパネル、ペン入力、スクリーンリーダーの音声読み上げ。Windows実データの索引は空だったため、結果Cardの状態は同じ実行バイナリのdeterministic mockとVitest / Playwrightで確認した。
- Review result:
  - 一貫性: Timelineと同じ`PostCard`、著者表示、公開範囲、時刻、context menu、通報dialogを再利用する。
  - ショートカット: 結果Cardから著者詳細、IDコピー、対象nodeへの通報へ直接移れる。
  - フィードバック: loading / error / emptyは既存表示を維持し、操作切替時は旧結果を即座に失効させる。
  - 完結性: 検索・発見・おすすめの全operationを同じ表示契約で扱う。
  - エラー防止: 取得できないデータや生のscope識別子を推測表示せず、read-onlyで成立しない操作を隠す。
  - 取り消し: IDコピーと通報dialogのopenは非破壊で、dialogは送信前にcancelできる。
  - 主導権: operation / node / scopeの変更で古い結果と古い通報対象を残さない。
  - 記憶負荷: Explore専用Cardの別ルールをなくし、既知のTimeline Card操作へ統一する。
- Exceptions: None

## 2026-08-31 仕様上書き

- PR #839 を Issue #820 の操作仕様に対する正式な上書きとする。上記の読み取り専用表示は PR #837 時点のレビュー記録として保持する。
- 「提供部分のみ有効化」は、索引レスポンスに含まれるデータだけを表示するという意味ではなく、「Community Index の結果から実処理を完了できる投稿アクションだけを表示し、有効化する」と定義する。
- 表示・有効化の条件には、必要な識別子とtopic / channel文脈、権限・capability、対象objectの解決、backend処理の成功、成功後の表示状態への反映を含む。callbackやAPIが配線されているだけでは動作可能とみなさない。
- 返信、リポスト、リアクション、リンクコピー、ブックマーク、通報、取り下げなど、共通 `PostCard` が提示し得るすべての投稿アクションをこの判定対象とする。
- 条件を満たせないアクションは表示しない。処理中の一時的な無効化は許容するが、恒常的に実行不能なボタンを無効状態で表示することは提供に含めない。
- 表示・有効化する各アクションは、Community Index の結果を起点として処理完了と画面反映までを自動テストまたは明記した手動確認で検証する。

## 2026-08-31 仕様上書きの実装レビュー

- Summary: remote Community Index 契約を変更せず、可視結果を desktop 内部で scope 単位に hydrate して正本 `PostView` と action capability を返す bounded batch 契約を追加した。解決中・対象欠落・scope 不一致・権限不足・親 topic 不明では fail-closed とし、実処理を完了できる action だけを `PostCard` へ渡す。
- Author resolution: 自分自身は `localProfile`、既知著者は既存 cache、未取得著者は重複排除した `getAuthorSocialView` で解決する。取得中、取得失敗、取得済みだが名前未設定を分離し、「不明な著者」は最後の状態だけに使用する。node / scope / operation が変わった後の古い応答は反映しない。
- Action safety: 返信対象が見つからない場合は通常投稿へ変化させず失敗する。リアクションとブックマークは操作時に scope と対象 projection を再 hydrate し、取り下げは durable docs の署名済み envelope を再解決して author / topic / channel を検証する。成功したリアクション、ブックマーク、取り下げ後は対象 Card を再解決する。
- Windows / WebView2: Windows 11 の Tauri development build を隔離 app data と keyring 無効化で起動した。実 runtime はネットワーク開始前の利用規約同意 gate まで正常表示し、同意操作は行わなかった。同じ Tauri / WebView2 へ deterministic mock を注入した確認では、Explore 検索結果の著者が `kukuri builder` と解決され、「不明な著者」にならないこと、リアクション・リポスト・返信・リンクコピー・ブックマーク・通報だけが表示されること、返信が対象の公開 dev thread と composer まで到達することを Computer Use で確認した。
- Validation:
  - failing-first: 返信対象欠落が通常投稿を作成する backend 回帰、正本 resolver 未呼出し・未解決 action 表示・著者未取得の frontend 回帰を追加テストで再現してから修正した。
  - `cargo xtask check`: formatting、workspace clippy、Tauri check、lint、typecheck passed。
  - `cargo xtask test`: Rust 700 passed / 3 skipped、harness 22 passed、frontend 131 files / 988 tests passed。
  - `cargo xtask desktop-ui-check`: lint、typecheck、frontend 988 tests、Storybook build、browser Playwright 52 tests、visual smoke 14 tests passed。
  - `cargo xtask tauri-check`、`cargo xtask e2e-smoke`、`cargo xtask oversized-files`、`git diff --check` passed。oversized-files は既存 baseline warning のみ。
- Not verified: 物理タッチパネル、ペン入力、スクリーンリーダーの音声読み上げ。Windows実 runtimeでは利用規約へ同意せずネットワーク通信を開始しなかったため、Community Index の処理完了経路は同じ Tauri / WebView2 の deterministic mock、Playwright、Vitest、Rust integration testsで確認した。
