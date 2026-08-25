# Issue #762 投稿コンテンツの権利表明と限定的利用許諾

## 概要

投稿コンテンツの権利が投稿者に残ることを明記し、kukuri client、選択された共有範囲の peer、各 Community Node に対する技術的な利用許諾を必要な範囲に限定した。app-level legal bundle は version 2 とし、version 1 に同意済みの利用者にも更新同意を要求する。

## 実装内容

- canonical 利用規約に、権利帰属、必要な権利・許諾の保有表明、非独占・無償・共有範囲限定の技術的許諾を追加した。
- 公開 topic と private channel／DM を区別し、接続補助や通知 hint が audience を拡張しないこと、広告・宣伝・生成 AI／機械学習への二次利用を許諾しないことを明記した。
- 撤回後は対応 client／node の将来処理を止める一方、法令上必要な限定保持と、peer が取得済みの copy を完全回収できない限界を区別した。
- Community Node の生成規約は、当該 node で有効かつ提供中の能力だけから索引・安全性走査・blob cache・private message 保管・暗号化 transit の許諾を生成する。無効な能力、他 node、network 全体には許諾を広げない。
- Legal UI の日本語・英語・中国語表示を揃え、legal bundle version 2 への更新同意が完了するまで従来どおり application shell を開始しない。

## スコープ境界

protocol、データフロー、Community Node の能力、DB schema、UI layout は変更していない。第三者端末に取得済みのデータを遠隔削除する機能や、network-wide に規約を強制する仕組みも追加していない。

## 検証

- `cargo xtask check`: 成功
- `cargo xtask test`: 成功（Rust nextest 592 件、harness 18 件、frontend 864 件を含む）
- `cargo xtask desktop-ui-check`: 成功（Playwright browser 35 件、visual 14 件を含む）
- `cargo xtask cn-check`: 成功
- `cargo xtask cn-test`: 成功
- Community Node 規約生成の能力別陽性／陰性試験と決定論的 golden: 成功
- legal bundle version 1 から version 2 への更新同意、日本語・英語・中国語の Legal UI、canonical 文書 version 契約試験: 成功
