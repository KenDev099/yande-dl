use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Rating {
    Safe,
    Questionable,
    Explicit,
}

impl Rating {
    pub fn from_moebooru_code(s: &str) -> Option<Self> {
        match s {
            "s" | "S" => Some(Self::Safe),
            "q" | "Q" => Some(Self::Questionable),
            "e" | "E" => Some(Self::Explicit),
            _ => None,
        }
    }

    pub fn to_short(self) -> &'static str {
        match self {
            Self::Safe => "s",
            Self::Questionable => "q",
            Self::Explicit => "e",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageVariant {
    pub url: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub size_bytes: Option<u64>,
    pub mime: Option<String>,
}

/// v0.1 only consumes `original` and `preview`. `sample` and `jpeg` are parsed
/// but unused (reserved for v0.2's "preview-before-download" and JPG mode).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostVariants {
    pub original: ImageVariant,
    pub preview: ImageVariant,
    #[serde(default)]
    pub sample: Option<ImageVariant>,
    #[serde(default)]
    pub jpeg: Option<ImageVariant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Post {
    pub provider_id: String,
    pub post_id: i64,
    /// MD5 of the original file's content. Not valid for jpeg/sample variants.
    pub md5: String,
    pub rating: Rating,
    pub score: i32,
    pub width: u32,
    pub height: u32,
    pub tags: Vec<String>,
    pub artist: Option<String>,
    pub source_url: Option<String>,
    pub created_at: Option<i64>,
    pub variants: PostVariants,
    /// Escape hatch for forward compatibility. v0.1 ignores this.
    #[serde(default)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchQuery {
    /// Tag list. Use `-tag` for Moebooru native exclusion.
    pub tags: Vec<String>,
    pub min_score: Option<i32>,
    pub min_width: Option<u32>,
    pub min_height: Option<u32>,
    pub ratings: Vec<Rating>,
    pub limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub max_results_per_page: u32,
    pub uses_md5: bool,
    /// Moebooru defaults to id-descending order; affects incremental termination logic.
    pub default_sort_desc_by_id: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rating_from_known_codes() {
        assert_eq!(Rating::from_moebooru_code("s"), Some(Rating::Safe));
        assert_eq!(Rating::from_moebooru_code("q"), Some(Rating::Questionable));
        assert_eq!(Rating::from_moebooru_code("e"), Some(Rating::Explicit));
    }

    #[test]
    fn rating_from_uppercase_codes() {
        assert_eq!(Rating::from_moebooru_code("S"), Some(Rating::Safe));
        assert_eq!(Rating::from_moebooru_code("E"), Some(Rating::Explicit));
    }

    #[test]
    fn rating_from_unknown_code() {
        assert_eq!(Rating::from_moebooru_code("x"), None);
        assert_eq!(Rating::from_moebooru_code(""), None);
    }

    #[test]
    fn rating_to_short_roundtrip() {
        for r in [Rating::Safe, Rating::Questionable, Rating::Explicit] {
            assert_eq!(Rating::from_moebooru_code(r.to_short()), Some(r));
        }
    }
}
