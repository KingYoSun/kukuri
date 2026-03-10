# Community Node Access Control P2P-only 再編

- v1 を P2P-only に再定義し、invite.capability + join.request(39022) で join できる設計に更新
- key.envelope 配布は direct P2P を正とし、relay/User API への依存を必須から解除
- User API の Access Control を外し、P2P-only を明確化
- 関連ドキュメント（KIP-0001/access_control_design/community_node_plan/user_api/topic_subscription/services_relay/event_treatment/architecture_overview/summary/billing/policy_consent/postgres/personal_data/roadmap）を整合更新
