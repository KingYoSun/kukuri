# Issue #856 利用規約の本番運用向け改訂

参照: Issue #856（親: #853）

実施日: 2026-09-03

## 完了内容

- `docs/legal/terms-of-service.md` を日本語正文として全面改訂し、定義、適用範囲、18歳以上の自己申告、アカウント・鍵管理、禁止事項、投稿者責任、知的財産、利用許諾、削除限界、Community Node、通報・制限、変更・終了、責任制限、準拠法・管轄、規約変更を明文化した。
- kukuri 運営者、利用者、投稿者、実際の受信利用者、Community Node 運営者、P2P 上の第三者を区別し、配布主体がネットワーク全体や第三者 Node を一元管理しない責任境界を固定した。
- 投稿コンテンツの権利は投稿者に残し、配信・複製に必要な範囲の非独占的許諾を実際の受信利用者と各 Community Node 運営者へ与える構成にした。端末やソフトウェア自体を権利主体として扱わない。
- 投稿削除や利用終了後も、既に受信・複製・保存された第三者 copy、cache、P2P 上のデータをネットワーク全体から回収・削除できるとは保証しないことを明記した。
- 秘密鍵の中央再発行・中央復旧を提供しない現行仕様と、export/import、backup/restore に関する利用者責任を規約へ反映した。
- MIT License によるソフトウェア利用許諾と、本規約によるサービス利用条件の関係を整理した。消費者契約法等の強行法規を妨げない責任制限、日本法準拠、東京地方裁判所または東京簡易裁判所の専属的合意管轄を定めた。
- legal bundle を version 5、施行日 2026-09-03 に更新し、既存 v4 同意では再同意を要求する。年齢自己申告 version は1のままとし、法務文書同意とは独立して保持する。
- ja／en／zh-CN のアプリ表示を同じ条項構成へ更新した。日本語のみを正文とし、英語・簡体字中国語は参考訳として表示する。
- privacy policy、external transmission notice、data flow inventory、consent／age-attestation data classification の bundle metadata を v5 に同期した。

## 契約

- failing-first: canonical legal document test と frontend の文書表示契約を先に v5 の必須条項へ更新し、旧規約で失敗することを確認してから正文・参考訳を実装した。
- backend contract で bundle version／施行日、配布 metadata の管理主体・窓口、利用規約の必須条項を固定した。
- `LegalDocumentView.test.tsx` で ja／en／zh-CN の主要見出し、v5、施行日を固定した。
- `App.test.tsx` で v4 から v5 への再同意と、再同意後も年齢自己申告が独立して維持されることを固定した。
- 配布主体・窓口は distribution metadata から注入し、operator-neutral な product source へ固有値を直書きしない。

## 検証

- `cargo xtask check`
- `cargo xtask test`（workspace 742 tests、harness 22 tests、frontend 140 files / 1074 tests）
- `cargo xtask desktop-ui-check`（lint、typecheck、frontend 140 files / 1074 tests、Storybook build、browser 58 tests、visual smoke 14 tests）
- `cargo xtask e2e-smoke`（desktop_smoke_post_persist、6 steps）
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml canonical_legal_documents_match_the_runtime_bundle_version -- --nocapture`
- targeted legal／consent／i18n tests 73件
- `git diff --check`

## 境界

- 本対応は現行実装・データフロー・配布形態との整合を対象とし、弁護士その他の専門家による法的レビュー済みであることを意味しない。
- 配布主体は、第三者 Community Node、P2P 接続相手、第三者の保存領域を一元管理せず、ネットワーク全体からの削除、受信済み copy の回収、秘密鍵の中央復旧を保証しない。
- Community Node 固有の利用条件、データ処理、保持、通報、制限、問い合わせは各 Node の manifest と公開文書に従い、app-level 規約同意とは別に記録する。
