# Issue #794 Metaverse resource budget 実装記録

## 完了した範囲

- Dome、player、host、clientの共通budget型、既定profile、安全hard ceiling、typed rejection、低cardinality metricsを追加した。
- TextureとGLB/VRMを実bytesからboundedに検査し、Preset asset metadataへ固定した。Import、Community Node staging、host start、client model previewで再検証する。
- Shared host runtimeへparticipant、prop/asset、collider/rigid body、input/interaction/spawn rate、impulse、snapshot頻度/帯域のmutation前preflightを追加した。Owner hostとCommunity Node hostは同じruntimeを使う。
- Desktop/CNのcache容量を共通設定へ移し、既存のpin、24時間grace、unpinned LRU規則を維持した。Connection proposalはproposer owner × receiver instance/slotの10分windowで制限する。
- Clientにdeterministic budget plannerを追加し、texture、remote avatar、prop interpolation、最大4隣接Domeを段階的にdegradeする。Hosting UIへ適用budget、metrics、利用者向け拒否理由を表示する。
- Metaverse world schemaを6へ更新した。実験機能のため旧schema migrationは提供しない。

## 後続Issueとの境界

隣接Domeの実prefetch/transitionとprop ownership移送は#790、access policyは#795、offline presence/編集は#796、entry/evacuation UXは#797が本budget contractを利用して実装する。
