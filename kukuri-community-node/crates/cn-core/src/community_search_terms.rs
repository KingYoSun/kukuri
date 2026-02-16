use std::collections::HashSet;

use crate::search_normalizer;

pub const COMMUNITY_TERM_TYPE_NAME: &str = "name";
pub const COMMUNITY_TERM_TYPE_ALIAS: &str = "alias";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommunitySearchTerm {
    pub term_type: &'static str,
    pub term_raw: String,
    pub term_norm: String,
    pub is_primary: bool,
}

pub fn community_id_from_topic_id(topic_id: &str) -> Option<String> {
    let trimmed = topic_id.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

pub fn build_terms_from_topic_id(topic_id: &str) -> Vec<CommunitySearchTerm> {
    let Some(community_id) = community_id_from_topic_id(topic_id) else {
        return Vec::new();
    };

    let mut raw_terms: Vec<(&'static str, String, bool)> =
        vec![(COMMUNITY_TERM_TYPE_NAME, community_id.clone(), true)];

    if let Some(stripped) = community_id.strip_prefix("kukuri:") {
        if !is_hashed_topic_tail(stripped) {
            raw_terms.push((COMMUNITY_TERM_TYPE_ALIAS, stripped.to_string(), true));
            if let Some(legacy_stripped) = stripped.strip_prefix("tauri:") {
                raw_terms.push((COMMUNITY_TERM_TYPE_ALIAS, legacy_stripped.to_string(), true));
            }
            if let Some(last_segment) = stripped.rsplit(':').next() {
                if !last_segment.is_empty() && last_segment != stripped {
                    raw_terms.push((COMMUNITY_TERM_TYPE_ALIAS, last_segment.to_string(), false));
                }
            }
        }
    }

    let mut seen = HashSet::new();
    let mut normalized_terms = Vec::new();
    for (term_type, term_raw, is_primary) in raw_terms {
        let term_norm = search_normalizer::normalize_search_text(&term_raw);
        if term_norm.is_empty() {
            continue;
        }

        let dedupe_key = format!("{term_type}:{term_norm}");
        if !seen.insert(dedupe_key) {
            continue;
        }

        normalized_terms.push(CommunitySearchTerm {
            term_type,
            term_raw,
            term_norm,
            is_primary,
        });
    }

    normalized_terms
}

fn is_hashed_topic_tail(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|ch| ch.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_terms_from_topic_id_generates_name_and_alias_terms() {
        let terms = build_terms_from_topic_id("kukuri:tauri:Rust-Dev");
        let norms: Vec<String> = terms.iter().map(|term| term.term_norm.clone()).collect();
        assert!(norms.contains(&"kukuri tauri rust dev".to_string()));
        assert!(norms.contains(&"tauri rust dev".to_string()));
        assert!(norms.contains(&"rust dev".to_string()));
    }

    #[test]
    fn build_terms_from_topic_id_skips_alias_for_hashed_tail() {
        let hashed_tail = "a".repeat(64);
        let topic_id = format!("kukuri:{hashed_tail}");
        let terms = build_terms_from_topic_id(&topic_id);

        assert_eq!(terms.len(), 1);
        assert_eq!(terms[0].term_type, COMMUNITY_TERM_TYPE_NAME);
        assert_eq!(terms[0].term_norm, format!("kukuri {hashed_tail}"));
    }

    #[test]
    fn community_id_from_topic_id_returns_none_for_empty_input() {
        assert!(community_id_from_topic_id("   ").is_none());
    }
}
