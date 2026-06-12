use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use regex::Regex;

use crate::model::{ClipboardItem, MatchRange, SearchMode, SearchResult};

const FUZZY_SEARCH_LIMIT: usize = 5_000;

pub struct SearchEngine;

impl SearchEngine {
    pub fn search(
        query: &str,
        items: &[ClipboardItem],
        mode: SearchMode,
    ) -> Vec<SearchResult> {
        if query.is_empty() {
            return items
                .iter()
                .map(|item| SearchResult {
                    item: item.clone(),
                    score: None,
                    ranges: vec![],
                })
                .collect();
        }

        match mode {
            SearchMode::Exact => Self::exact_search(query, items),
            SearchMode::Fuzzy => Self::fuzzy_search(query, items),
            SearchMode::Regexp => Self::regexp_search(query, items),
            SearchMode::Mixed => Self::mixed_search(query, items),
        }
    }

    fn exact_search(query: &str, items: &[ClipboardItem]) -> Vec<SearchResult> {
        items
            .iter()
            .filter_map(|item| {
                let title = &item.title;
                if let Some(byte_start) = title.to_lowercase().find(&query.to_lowercase()) {
                    let char_start = title[..byte_start].chars().count();
                    let matched_len = title[byte_start..]
                        .find(|c: char| !c.is_ascii())
                        .map_or(query.len(), |_| {
                            // Count chars in the matched range
                            let end_byte = byte_start + query.len();
                            if end_byte <= title.len() {
                                title[byte_start..end_byte].chars().count()
                            } else {
                                title[byte_start..].chars().count()
                            }
                        });
                    Some(SearchResult {
                        item: item.clone(),
                        score: None,
                        ranges: vec![MatchRange {
                            start: char_start as i64,
                            end: (char_start + matched_len) as i64,
                        }],
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    fn fuzzy_search(query: &str, items: &[ClipboardItem]) -> Vec<SearchResult> {
        let matcher = SkimMatcherV2::default().ignore_case();

        let mut results: Vec<SearchResult> = items
            .iter()
            .filter_map(|item| {
                let mut title = item.title.clone();
                if title.len() > FUZZY_SEARCH_LIMIT {
                    title.truncate(FUZZY_SEARCH_LIMIT);
                }

                matcher
                    .fuzzy_indices(&title, query)
                    .map(|(score, indices)| {
                        let ranges = char_indices_to_ranges(&indices);
                        SearchResult {
                            item: item.clone(),
                            score: Some(score as f64),
                            ranges,
                        }
                    })
            })
            .collect();

        // Sort by score (lower is better for SkimMatcherV2... but actually higher is better)
        results.sort_by(|a, b| {
            let sa = a.score.unwrap_or(0.0);
            let sb = b.score.unwrap_or(0.0);
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        });

        results
    }

    fn regexp_search(query: &str, items: &[ClipboardItem]) -> Vec<SearchResult> {
        let re = match Regex::new(query) {
            Ok(re) => re,
            Err(_) => return vec![],
        };

        items
            .iter()
            .filter_map(|item| {
                re.find(&item.title).map(|mat| {
                    let start = item.title[..mat.start()].chars().count();
                    let end = start + mat.as_str().chars().count();
                    SearchResult {
                        item: item.clone(),
                        score: None,
                        ranges: vec![MatchRange {
                            start: start as i64,
                            end: end as i64,
                        }],
                    }
                })
            })
            .collect()
    }

    fn mixed_search(query: &str, items: &[ClipboardItem]) -> Vec<SearchResult> {
        // Try exact first
        let results = Self::exact_search(query, items);
        if !results.is_empty() {
            return results;
        }

        // Try regex
        let results = Self::regexp_search(query, items);
        if !results.is_empty() {
            return results;
        }

        // Fall back to fuzzy
        Self::fuzzy_search(query, items)
    }
}

/// Convert byte indices from fuzzy matcher to character-based MatchRange list.
fn char_indices_to_ranges(indices: &[usize]) -> Vec<MatchRange> {
    if indices.is_empty() {
        return vec![];
    }

    let mut ranges = Vec::new();
    let mut start = indices[0] as i64;
    let mut end = start + 1;

    for &idx in &indices[1..] {
        let idx = idx as i64;
        if idx == end {
            end = idx + 1;
        } else {
            ranges.push(MatchRange { start, end });
            start = idx;
            end = idx + 1;
        }
    }
    ranges.push(MatchRange { start, end });

    ranges
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ClipboardContent;

    fn make_item(id: &str, title: &str) -> ClipboardItem {
        ClipboardItem {
            id: id.to_string(),
            application: None,
            first_copied_at: 0,
            last_copied_at: 0,
            number_of_copies: 1,
            pin: None,
            title: title.to_string(),
            contents: vec![ClipboardContent {
                content_type: "public.utf8-plain-text".to_string(),
                value: Some(title.as_bytes().to_vec()),
            }],
            sync_timestamp: 0,
            sync_source: None,
            sync_deleted: false,
        }
    }

    #[test]
    fn test_exact_search() {
        let items = vec![
            make_item("1", "Hello World"),
            make_item("2", "hello there"),
            make_item("3", "Goodbye"),
        ];
        let results = SearchEngine::search("hello", &items, SearchMode::Exact);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_fuzzy_search() {
        let items = vec![
            make_item("1", "Hello World"),
            make_item("2", "Help wanted"),
            make_item("3", "Goodbye"),
        ];
        let results = SearchEngine::search("hlo", &items, SearchMode::Fuzzy);
        assert!(!results.is_empty());
        assert_eq!(results[0].item.id, "1"); // "Hello" should match best
    }

    #[test]
    fn test_regexp_search() {
        let items = vec![
            make_item("1", "Hello World"),
            make_item("2", "hello there"),
            make_item("3", "Goodbye"),
        ];
        // "Hel+o" matches "Hello" (H-e-ll-o) but not "hello" (lowercase h)
        let results = SearchEngine::search("Hel+o", &items, SearchMode::Regexp);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].item.id, "1");
    }

    #[test]
    fn test_mixed_search_falls_through() {
        let items = vec![
            make_item("1", "Hello World"),
            make_item("2", "Goodbye"),
        ];
        // "hlw" won't match exact or regex, but should match fuzzy
        let results = SearchEngine::search("hlw", &items, SearchMode::Mixed);
        assert!(!results.is_empty());
    }

    #[test]
    fn test_empty_query_returns_all() {
        let items = vec![
            make_item("1", "Hello"),
            make_item("2", "World"),
        ];
        let results = SearchEngine::search("", &items, SearchMode::Exact);
        assert_eq!(results.len(), 2);
    }
}
