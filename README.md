English | [日本語](./README.ja.md)

# kukuri

kukuri is a topic-first P2P social app and protocol. It keeps Nostr-derived identity and signed envelope semantics where they are useful, but its internal sync plane is built around separated `docs`, `blobs`, `hints`, and connectivity instead of a relay-first design.

## Builder Preview

- Current preview target: Windows installer via GitHub Releases.
- Linux remains source-run for now.
- Preview flow: launch the desktop app, let the preloaded community node reach `ready`, open a starter topic, post or reply, then send feedback.
- Preview quickstart: [docs/runbooks/mvp-user-quickstart.md](./docs/runbooks/mvp-user-quickstart.md)
- Troubleshooting: [docs/runbooks/mvp-troubleshooting.md](./docs/runbooks/mvp-troubleshooting.md)

## What To Try In 3 Minutes

1. Launch the app and wait for the preloaded community node to become `ready`.
2. Open one of the starter topics: `kukuri:topic:demo`, `kukuri:topic:iroh`, `kukuri:topic:nostr`, or `kukuri:topic:operators`.
3. Publish a public post or reply in-thread.
4. Create or join a private channel under the same topic.
5. Export diagnostics and send feedback through GitHub before you close the app.

## What Community Nodes Do

- Community nodes are bootstrap, auth, control-plane, and connectivity-assist components.
- They help the app discover connectivity URLs and seed peers.
- They are not the canonical content store for user data.
- Each configured node stays part of the same `Community Node` model. Preview automation is controlled per node with an `auto_approve` policy, not with a separate official/custom split.

## What kukuri Is Not

- kukuri is not a full Nostr client.
- Nostr compatibility is limited to identity, envelope shape, and selected semantics.
- kukuri is not relay-first internally.
- Community nodes are not relays with canonical content ownership.

## What Is kukuri

- Topics are the main browsing and publishing surface.
- Channels are audience scopes under a topic, not standalone workspaces.
- The same design aims to cover public timelines, private channels, pairwise DM, live sessions, and game rooms.
- Users keep local key ownership and publish signed objects from their own identity.

## Core Concepts

- Signed envelopes are proof and metadata, not the whole data plane.
- Hints are notifications and sync triggers, not a source of truth.
- Structured state is synchronized through `docs`.
- Media and other large payloads are synchronized through `blobs`.
- Connectivity comes from static-peer links, seeded DHT discovery, and community-node assist.
- Durability is designed around offline use, restart recovery, and late join backfill.
- Community nodes are bootstrap, auth, control-plane, and connectivity-assist components, not the canonical store for user content.
- Nostr compatibility is a limited subset for identity, envelope shape, and some tags; kukuri's internal sync model is its own.

## What Works Today

- Desktop targets: Linux and Windows.
- Connectivity: static-peer, seeded DHT discovery, and community-node connectivity/auth.
- Topic timeline flow: public posts, reply/thread, image attachments, and video attachments.
- Social graph v1: public profiles, follow/unfollow, `mutual`, and `friend of friend` display.
- Private channel audience v1: `invite_only`, `friend_only`, and `friend_plus` with epoch-aware lifecycle.
- Pairwise DM v1: 1:1, mutual-only, offline-capable, local transcript/delete, and image/video attachments.
- Live session and game room state that recover from `docs + blobs`.

Current scope is defined by [foundation progress](./docs/progress/2026-03-10-foundation.md) and the accepted ADRs under [docs/adr/](./docs/adr/).

## Longer-Term Direction

- Search and suggestion can be provided by optional specialized services instead of becoming required parts of the core sync plane.
- Gateway and bridge layers can provide selective import/export and ecosystem interoperability.
- Trust, moderation, and policy-assist functions can grow around community nodes without turning them into the canonical content store.
- These are planned directions and optional ecosystem services, not a statement that they are all shipped in the current workspace.

## For Contributors

- New work targets the root workspace.
- Current sources of truth:
  - [docs/progress/2026-03-10-foundation.md](./docs/progress/2026-03-10-foundation.md)
  - [docs/README.md](./docs/README.md)
  - [docs/runbooks/dev.md](./docs/runbooks/dev.md)
  - [harness/scenarios/](./harness/scenarios/)
- Key protocol and product references:
  - [docs/adr/0010-kukuri-protocol-v1-boundary-definition.md](./docs/adr/0010-kukuri-protocol-v1-boundary-definition.md)
  - [docs/adr/0011-kukuri-protocol-v1-draft.md](./docs/adr/0011-kukuri-protocol-v1-draft.md)
  - [docs/adr/0012-topic-first_progressive_community_filtering_draft.md](./docs/adr/0012-topic-first_progressive_community_filtering_draft.md)
  - [docs/adr/0013-social-graph-foundation-draft.md](./docs/adr/0013-social-graph-foundation-draft.md)
  - [docs/adr/0018-channel-first-sidebar-and-unified-epoch-lifecycle.md](./docs/adr/0018-channel-first-sidebar-and-unified-epoch-lifecycle.md)
  - [docs/adr/0020-pairwise-dm-v1.md](./docs/adr/0020-pairwise-dm-v1.md)

### Entry Points

```bash
cargo xtask doctor
cargo xtask check
cargo xtask test
cargo xtask e2e-smoke

cd apps/desktop
npx pnpm@10.16.1 install
npx pnpm@10.16.1 dev
```

For day-to-day commands and validation paths, use [docs/runbooks/dev.md](./docs/runbooks/dev.md).

## License

MIT
