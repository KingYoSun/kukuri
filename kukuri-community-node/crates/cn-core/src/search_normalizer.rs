use std::collections::HashSet;
use unicode_normalization::UnicodeNormalization;

pub const SEARCH_NORMALIZER_VERSION: i16 = 1;

pub fn normalize_search_text(value: &str) -> String {
    if value.trim().is_empty() {
        return String::new();
    }

    let mut normalized = String::with_capacity(value.len());
    for ch in value.nfkc().flat_map(char::to_lowercase) {
        if ch.is_control() || ch.is_whitespace() {
            normalized.push(' ');
            continue;
        }

        if ch == '#' || ch == '@' || ch.is_alphanumeric() {
            normalized.push(ch);
        } else {
            normalized.push(' ');
        }
    }

    normalized.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn normalize_search_terms<I, S>(values: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();

    for value in values {
        let term = normalize_search_text(value.as_ref());
        if term.is_empty() {
            continue;
        }

        for segment in term.split_whitespace() {
            if seen.insert(segment.to_string()) {
                normalized.push(segment.to_string());
            }
        }
    }

    normalized
}

pub fn build_search_text(
    body_norm: &str,
    hashtags_norm: &[String],
    mentions_norm: &[String],
    community_terms_norm: &[String],
) -> String {
    let mut parts = Vec::new();
    if !body_norm.is_empty() {
        parts.push(body_norm.to_string());
    }
    parts.extend(hashtags_norm.iter().cloned());
    parts.extend(mentions_norm.iter().cloned());
    parts.extend(community_terms_norm.iter().cloned());
    normalize_search_text(&parts.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_search_text_applies_nfkc_casefold_and_symbol_rules() {
        let input = "Ｈｅｌｌｏ　Ｗｏｒｌｄ!!! #Ｒｕｓｔ @ＡＬＩＣＥ";
        assert_eq!(normalize_search_text(input), "hello world #rust @alice");
    }

    #[test]
    fn normalize_search_text_handles_multilingual_and_emoji() {
        let input = "Rust🚀\nテスト　かな\t中文!";
        assert_eq!(normalize_search_text(input), "rust テスト かな 中文");
    }

    #[test]
    fn normalize_search_terms_deduplicates_and_skips_empty_values() {
        let terms = normalize_search_terms([" #Rust ", "rust", " @Alice ", "", "ＲＵＳＴ"]);
        assert_eq!(terms, vec!["#rust", "rust", "@alice"]);
    }

    #[test]
    fn build_search_text_merges_all_normalized_sources() {
        let body = normalize_search_text("Hello, 世界");
        let hashtags = normalize_search_terms(["#Rust"]);
        let mentions = normalize_search_terms(["@Alice"]);
        let communities = normalize_search_terms(["kukuri:topic-one"]);

        assert_eq!(
            build_search_text(&body, &hashtags, &mentions, &communities),
            "hello 世界 #rust @alice kukuri topic one"
        );
    }
}
