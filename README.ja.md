日本語 | [English](./README.md)

# kukuri

kukuri は topic-first な P2P social app / protocol です。Nostr 由来の署名付き identity と envelope の利点は活かしつつ、内部の同期モデルは relay-first ではなく、`docs`、`blobs`、`hints`、connectivity を分離した構成を中核に据えています。

## kukuri とは何か

- topic が閲覧と発信の主軸です。
- channel は topic 配下の audience / scope であり、独立した workspace ではありません。
- public timeline、private channel、pairwise DM、live session、game room を同じ設計思想で扱います。
- ユーザーは鍵をローカルに保持し、自分の identity で signed object を発行します。

## 重要な設計原則

- signed envelope は証明とメタデータであり、データプレーン全体そのものではありません。
- hint は通知と同期のきっかけであり、source of truth ではありません。
- structured state は `docs` で同期します。
- media や大きな payload は `blobs` で同期します。
- connectivity は static-peer、seeded DHT discovery、community-node assist が担います。
- durability は offline 利用、restart 復元、late join backfill を前提に設計します。
- community-node は bootstrap、auth、control plane、connectivity assist を担うもので、ユーザーコンテンツの canonical store ではありません。
- Nostr 互換は identity、envelope 形状、一部 tag に限った subset であり、kukuri の内部同期モデルは kukuri 固有です。

## 現在動いている範囲

- desktop target: Linux / Windows
- connectivity: static-peer、seeded DHT discovery、community-node connectivity/auth
- topic timeline: public post、reply/thread、image 添付、video 添付
- social graph v1: public profile、follow/unfollow、`mutual`、`friend of friend` 表示
- private channel audience v1: `invite_only`、`friend_only`、`friend_plus` と epoch-aware lifecycle
- pairwise DM v1: 1on1、mutual 限定、offline 可、local transcript/delete、image/video attachment
- `docs + blobs` から復元できる live session / game room state

現在のスコープは [foundation progress](./docs/progress/2026-03-10-foundation.md) と [docs/adr/](./docs/adr/) 配下の accepted ADR を正とします。

## 今後の方向性

- 検索やサジェストは、core sync plane に必須で埋め込むのではなく、optional な specialized service として扱えるようにする方針です。
- gateway / bridge 層によって、選択的な import/export や ecosystem interoperability を持てる余地を残します。
- trust、moderation、policy assist は community-node の周辺で拡張しうる一方、canonical content store にはしません。
- これらは longer-term direction / optional ecosystem services であり、現行 workspace ですべて出荷済みという意味ではありません。

## コントリビューター向け

- 新規実装・修正は root workspace を対象にします。
- `legacy/` は、明示的な移植タスクがない限り参照専用です。
- 現在の truth は主に次です。
  - [docs/progress/2026-03-10-foundation.md](./docs/progress/2026-03-10-foundation.md)
  - [docs/README.md](./docs/README.md)
  - [docs/runbooks/dev.md](./docs/runbooks/dev.md)
  - [harness/scenarios/](./harness/scenarios/)
- protocol / product の主要参照は次です。
  - [docs/adr/0010-kukuri-protocol-v1-boundary-definition.md](./docs/adr/0010-kukuri-protocol-v1-boundary-definition.md)
  - [docs/adr/0011-kukuri-protocol-v1-draft.md](./docs/adr/0011-kukuri-protocol-v1-draft.md)
  - [docs/adr/0012-topic-first_progressive_community_filtering_draft.md](./docs/adr/0012-topic-first_progressive_community_filtering_draft.md)
  - [docs/adr/0013-social-graph-foundation-draft.md](./docs/adr/0013-social-graph-foundation-draft.md)
  - [docs/adr/0018-channel-first-sidebar-and-unified-epoch-lifecycle.md](./docs/adr/0018-channel-first-sidebar-and-unified-epoch-lifecycle.md)
  - [docs/adr/0020-pairwise-dm-v1.md](./docs/adr/0020-pairwise-dm-v1.md)

### 作業入口

```bash
cargo xtask doctor
cargo xtask check
cargo xtask test
cargo xtask e2e-smoke

cd apps/desktop
npx pnpm@10.16.1 install
npx pnpm@10.16.1 dev
```

日常コマンドや検証手順の詳細は [docs/runbooks/dev.md](./docs/runbooks/dev.md) を参照してください。

## ライセンス

MIT
