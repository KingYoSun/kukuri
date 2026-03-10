# friend_plus v1 方針（FoF + pull join.request）
- friend を kind=3 相互フォローで定義し、friend_plus は FoF(2-hop) を pull join.request で受理する方針に決定
- Key Steward 自動配布は使わず、受信側が FoF 判定後に key.envelope を配布する前提へ整理
- Access Control 設計/KIP-0001/community_node_plan/community_nodes_roadmap/summary を整合
