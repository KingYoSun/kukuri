/// #1061: ブロック・ミュート観測の提供に使う任意同意文書の slug。
///
/// Rust 側の `kukuri_cn_protocol::TRUST_OBSERVATION_SHARING_POLICY_SLUG` と同じ値。この文書は
/// CN 設定の専用トグルでだけ同意するため、通常の同意ダイアログの一覧・一括受諾には含めない
/// （ADR 0026 §8.5）。
export const TRUST_OBSERVATION_SHARING_POLICY_SLUG = 'trust_observation_sharing';
