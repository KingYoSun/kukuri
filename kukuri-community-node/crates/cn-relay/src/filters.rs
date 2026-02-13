use anyhow::{anyhow, Result};
use serde_json::Value;
use std::collections::HashMap;

const MAX_FILTERS: usize = 10;
const MAX_FILTER_VALUES: usize = 200;
const MAX_LIMIT: i64 = 1000;

#[derive(Clone, Debug)]
pub struct RelayFilter {
    pub ids: Option<Vec<String>>,
    pub authors: Option<Vec<String>>,
    pub kinds: Option<Vec<u32>>,
    pub since: Option<i64>,
    pub until: Option<i64>,
    pub limit: Option<i64>,
    pub tags: HashMap<String, Vec<String>>,
}

impl RelayFilter {
    pub fn topic_ids(&self) -> Option<&Vec<String>> {
        self.tags.get("t")
    }
}

pub fn parse_filters(values: &[Value]) -> Result<Vec<RelayFilter>> {
    if values.is_empty() {
        return Err(anyhow!("missing filters"));
    }
    if values.len() > MAX_FILTERS {
        return Err(anyhow!("too many filters"));
    }

    let mut filters = Vec::new();
    for value in values {
        let map = value
            .as_object()
            .ok_or_else(|| anyhow!("filter must be an object"))?;

        let ids = parse_string_list(map.get("ids"))?;
        let authors = parse_string_list(map.get("authors"))?;
        let kinds = parse_u32_list(map.get("kinds"))?;
        let since = map.get("since").and_then(|v| v.as_i64());
        let until = map.get("until").and_then(|v| v.as_i64());
        let limit = map
            .get("limit")
            .and_then(|v| v.as_i64())
            .map(|value| value.clamp(1, MAX_LIMIT));

        let mut tags = HashMap::new();
        for (key, value) in map {
            if !key.starts_with('#') {
                continue;
            }
            let tag = key.trim_start_matches('#').to_string();
            let values = parse_string_list(Some(value))?.unwrap_or_default();
            if values.len() > MAX_FILTER_VALUES {
                return Err(anyhow!("too many values for tag {tag}"));
            }
            tags.insert(tag, values);
        }

        if tags
            .get("t")
            .map(|values| values.is_empty())
            .unwrap_or(true)
        {
            return Err(anyhow!("missing #t filter"));
        }

        filters.push(RelayFilter {
            ids,
            authors,
            kinds,
            since,
            until,
            limit,
            tags,
        });
    }

    Ok(filters)
}

fn parse_string_list(value: Option<&Value>) -> Result<Option<Vec<String>>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let list = value
        .as_array()
        .ok_or_else(|| anyhow!("expected array"))?
        .iter()
        .filter_map(|item| item.as_str().map(|s| s.to_string()))
        .collect::<Vec<_>>();
    if list.len() > MAX_FILTER_VALUES {
        return Err(anyhow!("too many filter values"));
    }
    Ok(Some(list))
}

fn parse_u32_list(value: Option<&Value>) -> Result<Option<Vec<u32>>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let list = value
        .as_array()
        .ok_or_else(|| anyhow!("expected array"))?
        .iter()
        .filter_map(|item| item.as_u64().and_then(|v| u32::try_from(v).ok()))
        .collect::<Vec<_>>();
    if list.len() > MAX_FILTER_VALUES {
        return Err(anyhow!("too many filter values"));
    }
    Ok(Some(list))
}

pub fn matches_filter(filter: &RelayFilter, event: &cn_core::nostr::RawEvent) -> bool {
    if let Some(ids) = &filter.ids {
        if !ids.iter().any(|id| event.id.starts_with(id)) {
            return false;
        }
    }
    if let Some(authors) = &filter.authors {
        if !authors
            .iter()
            .any(|author| event.pubkey.starts_with(author))
        {
            return false;
        }
    }
    if let Some(kinds) = &filter.kinds {
        if !kinds.contains(&event.kind) {
            return false;
        }
    }
    if let Some(since) = filter.since {
        if event.created_at < since {
            return false;
        }
    }
    if let Some(until) = filter.until {
        if event.created_at > until {
            return false;
        }
    }

    for (tag, values) in &filter.tags {
        let event_values = event.tag_values(tag);
        if !values.iter().any(|value| event_values.contains(value)) {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_filters_rejects_missing_topic_filter_with_stable_reason() {
        let err = parse_filters(&[json!({ "kinds": [1] })]).expect_err("must reject without #t");
        assert_eq!(err.to_string(), "missing #t filter");
    }

    #[test]
    fn parse_filters_rejects_too_many_filters_with_stable_reason() {
        let filters = (0..=MAX_FILTERS)
            .map(|_| json!({ "#t": ["kukuri:global"] }))
            .collect::<Vec<_>>();
        let err = parse_filters(&filters).expect_err("must reject too many filters");
        assert_eq!(err.to_string(), "too many filters");
    }

    #[test]
    fn parse_filters_rejects_too_many_filter_values_with_stable_reason() {
        let authors = (0..=MAX_FILTER_VALUES)
            .map(|i| format!("pubkey-{i}"))
            .collect::<Vec<_>>();
        let err = parse_filters(&[json!({
            "#t": ["kukuri:global"],
            "authors": authors
        })])
        .expect_err("must reject too many values");
        assert_eq!(err.to_string(), "too many filter values");
    }

    #[test]
    fn parse_filters_clamps_limit_to_maximum() {
        let filters = parse_filters(&[json!({
            "#t": ["kukuri:global"],
            "limit": MAX_LIMIT + 100
        })])
        .expect("filter should parse");
        assert_eq!(filters.len(), 1);
        assert_eq!(filters[0].limit, Some(MAX_LIMIT));
    }
}
