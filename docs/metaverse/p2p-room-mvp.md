# P2P Room Metaverse MVP

## 目的
この MVP は、kukuri が中央集権的なゲームサーバーを前提にせず、トピック単位の小さな 3D ソーシャルルームを扱えることを検証するためのものです。VRChat クローン、MMO、音声、ワールドエディタ、大規模リアルタイムシミュレーションは対象外です。

## アーキテクチャ選択
`docs/progress/2026-05-27-metaverse-mvp-plan.md` の Option A として、既存の game room model を拡張しています。

理由:

- game room は docs の状態ポインタと blob manifest という永続化パターンをすでに持っている。
- `game_room_cache` により、トピック単位の room discovery と再起動後の復元を使える。
- 既存の list/create/update 経路、private channel scope、`SessionChanged` hint を再利用できる。
- `room_kind` の既定値を `score_game` にしているため、既存の score/status game room と互換性を保てる。

## 実装済み範囲
- `GameRoomManifestBlobV1.room_kind`: `score_game` または `metaverse_room`。
- `GameRoomManifestBlobV1.metaverse`: world version、spawn、asset refs、共有オブジェクト 1 個を持つ MVP state。
- `GameRoomView.manifest_blob_hash`: UI から確認できる永続化/debug 用のシグナル。
- `create_metaverse_room` / `update_metaverse_room` Tauri command。
- Desktop の game section で通常の score room と metaverse room を分離表示。
- Metaverse discovery panel で、トピック単位の metaverse room 一覧と作成を提供。
- Room view で three.js scene、camera、lights、ground、local avatar、remote avatar、共有オブジェクトを描画。
- `apps/desktop/public/blumochichi.vrm` を local avatar としてロードし、失敗時は primitive fallback を使用。
- 任意の VRM file または sample VRM を blob storage に import し、`MetaverseAssetRef` として扱う。
- avatar presence は `avatar_asset_ref` を signed room event に載せ、raw bytes を room event に埋め込まない。
- avatar transform、chat、object update は署名済み `metaverse-room-event` envelope として hint transport で送受信する。
- WASD / arrow key movement により avatar transform event を約 10 Hz で送出。
- 共有オブジェクトの移動は `update_metaverse_room` 経由で manifest blob を更新する。

## Production MVP 完了条件
この goal は、以下を満たす状態を production MVP として扱う。

- metaverse room discovery は既存 game room list/projection を使い、複数 room を topic 単位で表示できる。
- room metadata と shared object state は docs pointer + manifest blob で永続化され、restart 後に復元できる。
- avatar transform、chat、object update は署名済み `metaverse-room-event` envelope として P2P hint transport で送受信できる。
- avatar transform は high-frequency な ephemeral event として扱い、docs/blobs に 10 Hz で直接書き込まない。
- VRM asset bytes は blob storage に入り、presence / room event には `MetaverseAssetRef` だけを載せる。
- `cargo xtask check` と `cargo xtask test` が最終状態で完走する。

## Kukuri Primitive との対応
- Hints: `SessionChanged` は room metadata の通知と同期トリガー。`MetaverseRoomEvent` は署名済み room event envelope の軽量 transport。
- Docs: topic/private-channel replica に game/metaverse room state pointer を保存。
- Blobs: manifest JSON、VRM/GLB などの asset bytes を保存。
- SQLite projection: `game_room_cache` に room discovery fields、room kind、manifest hash、metaverse JSON を保存。
- Connectivity/community node: optional facilitator として扱い、canonical authority にはしない。
- Avatar transforms: high-frequency な ephemeral event として扱い、docs/blobs に直接 10 Hz で書き込まない。

## 起動
```powershell
cargo xtask check
cd apps/desktop
npx pnpm@10.16.1 dev
```

desktop app を開き、`Game` に切り替えて `Metaverse Rooms` panel を使う。

## 1 クライアント確認
1. `Game` を開く。
2. metaverse room を作成する。
3. room card を開く。
4. 3D viewport が表示され、local avatar に `blumochichi.vrm` が読み込まれることを確認する。
5. `Sample VRM` または `VRM file` で avatar asset を blob に import し、`Avatar asset` が `blob VRM loaded` になることを確認する。
6. WASD または arrow key で local avatar を移動する。
7. room chat message を送信する。
8. object controls で共有オブジェクトを移動する。
9. refresh または再起動後、room list と共有オブジェクト state が manifest から復元されることを確認する。

## 2 クライアント確認
1. 通常の kukuri peer connectivity で desktop instance を 2 つ起動する。
2. 既存 game-room smoke flow と同じ topic と peer connectivity path を使う。
3. client A で metaverse room を作成する。
4. client B が room を discover できることを確認する。
5. 両 client で room を開く。
6. client A の avatar movement が client B の remote avatar に反映されることを確認する。
7. chat message が相互に送受信されることを確認する。
8. 共有オブジェクト更新が room event と manifest 経路で反映されることを確認する。

自動検証として、`metaverse_room_events_replicate_between_iroh_peers` は 2 つの Iroh peer 間で signed room event が配送されることを確認する。

## VRM / Asset Flow 確認
- local avatar は public sample または blob-backed VRM URL を `GLTFLoader` と `VRMLoaderPlugin` で読み込む。
- 読み込み成功時、debug panel の `Avatar asset` は `sample VRM loaded` または `blob VRM loaded` になる。
- 読み込み失敗時、debug panel は fallback 状態を表示し、primitive avatar を維持する。
- asset bytes は blob storage に入り、room event / presence には `MetaverseAssetRef` だけを載せる。

## 共有オブジェクト同期確認
共有オブジェクトの移動は `update_metaverse_room` を呼び、manifest blob と projection state を更新する。MVP の conflict rule は update time による last-write-wins とする。

自動検証として、`metaverse_room_manifest_restores_after_restart_from_docs_and_blobs` は restart 後に room list と shared object state が docs + blob から復元されることを確認する。

## 既知の制約
- 大規模同時接続、voice、WebRTC/SFU、ワールドエディタ、UGC scripting、asset safety scanning は goal の non-goal。
- chat は room-scoped signed event として扱う。topic timeline post として永続表示しないため、長期履歴 UI はこの MVP の対象外。

## Fallback / Mock と MVP 対象外
- desktop/Tauri 実行時の canonical path は `metaverse-room-event` envelope を使う signed P2P hint transport。
- browser-only dev shell では Tauri backend がないため、同一ブラウザ内確認用に `BroadcastChannel` fallback を残している。
- VRM が読み込めない場合でも room view を壊さず、primitive avatar を表示する。
- remote peer が avatar asset blob を解決できない場合も fallback avatar で表示を継続する。
- 長期 chat history、moderation UI、asset safety scanning、asset pinning/backfill の完全自動化はこの MVP の未実装範囲。

## 次の実装候補
- room event buffer を既存 message/history primitive と統合し、chat の長期履歴 UI を追加する。
- VRM/GLB asset の pinning/backfill 状態を debug panel から操作・確認できるようにする。
- remote avatar の補間と stale/leave 表示を polish する。
- 2 desktop instance の手動確認を e2e harness 化し、3D viewport のスクリーンショット検証も自動化する。
