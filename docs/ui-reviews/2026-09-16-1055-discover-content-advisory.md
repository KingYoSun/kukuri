# 2026-09-16 「見つける」の content advisory 代替表示

- Status: superseded
- Supersedes: None
- Superseded by: [2026-09-17 推定による代替表示の詳細 dialog 化](2026-09-17-1108-advisory-details-dialog.md)
- PR: [#1071](https://github.com/KingYoSun/kukuri/pull/1071)
- Issue / Scope revision: [#1055](https://github.com/KingYoSun/kukuri/issues/1055)、2026-09-15
- Preview: 下表の before / after 画像（`assets/1055/`）
- 対象 surface / 利用者 / 目的: 「見つける」Column（検索・発見・おすすめ）の閲覧者のうち、成人向け表現の表示設定が OFF の人。設定済みコミュニティノードが成人向けの可能性を推定した投稿を、投稿者自己申告の場合と同じ代替表示にし、その推定が誰の・何に基づく判断かを読めるようにする。
- 変更分類: 既存画面の改善（ADR 0014 §2）。state 追加と共有 component の拡張。

## 採用した表示

代替表示そのものは投稿者自己申告の場合と同じ枠を使う。本文は文言だけを差し替え、メディアは既存の `PostMedia` のプレースホルダーをそのまま使う。「見つける」専用の代替表示は作らない。

その下に説明ブロックを 1 つ足す。見出しは「コミュニティノードによる推定」で、続けて発行元・分類・確信度・根拠を定義リストで示し、最後に異議申し立てのボタンを置く。断定表現は使わず、本文でも説明でも「推定」と書き、投稿者の申告でもネットワーク全体の判断でもないことを明示する。

発行元は manifest の表示名を使い、その右に短縮した node_id を添える。manifest を取得できない場合は base URL の host へ落とし、説明ブロック自体は出す。異議申し立ては既存の通報ダイアログをそのまま申し立てモードで開き、送信先は当の推定を発行したノードだけに限られる。

canonical 解決が終わっていない結果でも代替表示にする。ただしまだ隠すべき本文が無いので、本文欄は従来の「解決中」の文言を保つ。添付を持たないため、メディア枠は出ない。

表示設定を ON にすると通常表示へ戻り、説明ブロックも消える。

## 条件と証跡

- Platform: Chromium（Playwright、deterministic mock）。Windows WebView2 実機は未確認（下記）。
- Viewport: 1400×980 と 390×844。
- Theme: dark と light。
- Locale: ja（自動テストは en も通る。zh-CN は文言のみ追加し実画面は未撮影）。
- State: advisory 付き解決済み（表示 OFF / ON）、advisory 付き未解決、manifest 取得失敗、未知ラベル、自己申告のみ。

| 条件 | 変更前 | 変更後 |
| --- | --- | --- |
| ja / dark / 1400px | ![before](assets/1055/before-ja-dark-1400-advisory-gated.png) | ![after](assets/1055/after-ja-dark-1400-advisory-gated.png) |
| ja / light / 390px | ![before](assets/1055/before-ja-light-390-advisory-gated.png) | ![after](assets/1055/after-ja-light-390-advisory-gated.png) |
| 異議申し立て導線 | 変更前は導線自体が無い | ![after](assets/1055/after-ja-dark-1400-advisory-appeal.png) |

変更前は同じ応答を受け取っても推定が無視され、画像がそのまま表示されていた。変更後はプレースホルダーと説明になり、メディアのバイト列を要求しない。

## Accessibility・性能・未確認事項

異議申し立てのボタンはカード全体を開く操作に飲み込まれず、キーボードで focus でき、実操作でダイアログが開くことを Chromium で確認した。説明ブロックは定義リストで、項目名と値が対で読める。狭幅 390px で横スクロールが発生しないことを同じ spec で確認した。node_id は折り返し可能にしてカード幅を超えない。

追加の polling は無い。manifest の取得は、gating 対象の推定を含む結果を受けたときにだけ 1 回行う。プリフェッチは対象の hash を除外集合に入れるだけで、新しい取得は増えない。大規模一覧の性能計測は本変更では行わない。

未確認: Windows WebView2 と Ubuntu WebKitGTK の実機描画、物理タッチ・ペン入力、screen reader の読み上げ、zh-CN の実画面。視覚回帰 baseline（Linux/Chromium）には新しい surface を追加していない。

## Review result

- 一貫性: 自己申告の場合と同じ枠・同じ component・同じ設定で切り替わる。ラベル源が 2 つになっても代替表示は 1 種類のまま。
- エラー防止: 推定を断定として書かず、発行元を必ず併記する。未知のラベルでは隠さない。発行元を確認できない推定は採用しない。
- 主導権: 推定に納得できない場合の申し立て導線が同じカード内にあり、送信先は発行元ノードだけに限られる。
- 記憶負荷: 説明は 4 項目に絞り、識別子は短縮して出す。

## Exceptions

None。必要な確認の未実施は「未確認事項」に記載した。
