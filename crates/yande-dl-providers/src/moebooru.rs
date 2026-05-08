use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use std::collections::HashMap;
use yande_dl_core::error::CoreError;
use yande_dl_core::model::{Capabilities, ImageVariant, Post, PostVariants, Rating, SearchQuery};
use yande_dl_core::provider::ImageProvider;
use yande_dl_core::retry::{with_backoff, RetryPolicy};

pub struct MoebooruProvider {
    id: String,
    display_name: String,
    base_url: String,
    client: Client,
}

impl MoebooruProvider {
    pub fn yandere(client: Client) -> Self {
        Self {
            id: "yande".into(),
            display_name: "Yande.re".into(),
            base_url: "https://yande.re".into(),
            client,
        }
    }

    pub fn konachan(client: Client) -> Self {
        Self {
            id: "konachan".into(),
            display_name: "Konachan".into(),
            base_url: "https://konachan.com".into(),
            client,
        }
    }

    /// Test/internal constructor that lets the caller override the base URL
    /// (e.g. point at a wiremock server).
    pub fn with_base_url(
        id: impl Into<String>,
        display_name: impl Into<String>,
        base_url: impl Into<String>,
        client: Client,
    ) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            base_url: base_url.into(),
            client,
        }
    }

    pub fn build_tag_string(query: &SearchQuery) -> String {
        let mut parts: Vec<String> = query.tags.clone();

        // Only emit `rating:` filters when we have a strict subset.
        if !query.ratings.is_empty() && query.ratings.len() < 3 {
            for r in &query.ratings {
                parts.push(format!("rating:{}", r.to_short()));
            }
        }
        if let Some(s) = query.min_score {
            parts.push(format!("score:>={}", s));
        }
        if let Some(w) = query.min_width {
            parts.push(format!("width:>={}", w));
        }
        if let Some(h) = query.min_height {
            parts.push(format!("height:>={}", h));
        }
        parts.join(" ")
    }

    async fn fetch_page(
        &self,
        tag_str: &str,
        page: u32,
        limit: u32,
    ) -> Result<Vec<MoebooruRawPost>, CoreError> {
        let url = format!("{}/post.json", self.base_url);
        let resp = self
            .client
            .get(&url)
            .query(&[
                ("tags", tag_str.to_string()),
                ("page", page.to_string()),
                ("limit", limit.to_string()),
            ])
            .send()
            .await?;

        let status = resp.status();
        if status == StatusCode::TOO_MANY_REQUESTS {
            let retry = resp
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok());
            return Err(CoreError::RateLimited {
                retry_after_secs: retry,
            });
        }
        if !status.is_success() {
            return Err(CoreError::Server {
                status: status.as_u16(),
            });
        }

        resp.json::<Vec<MoebooruRawPost>>()
            .await
            .map_err(|e| CoreError::Parse(e.to_string()))
    }

    fn normalize(&self, r: MoebooruRawPost) -> Post {
        let rating = Rating::from_moebooru_code(&r.rating).unwrap_or(Rating::Safe);
        let tags: Vec<String> = r.tags.split_whitespace().map(|s| s.to_string()).collect();

        Post {
            provider_id: self.id.clone(),
            post_id: r.id,
            md5: r.md5,
            rating,
            score: r.score,
            width: r.width,
            height: r.height,
            tags,
            artist: r.author,
            source_url: r.source,
            created_at: r.created_at,
            variants: PostVariants {
                original: ImageVariant {
                    url: r.file_url,
                    width: Some(r.width),
                    height: Some(r.height),
                    size_bytes: r.file_size,
                    mime: r.file_ext.as_deref().map(|e| format!("image/{}", e)),
                },
                preview: ImageVariant {
                    url: r.preview_url,
                    width: r.preview_width,
                    height: r.preview_height,
                    size_bytes: None,
                    mime: None,
                },
                sample: r.sample_url.map(|url| ImageVariant {
                    url,
                    width: r.sample_width,
                    height: r.sample_height,
                    size_bytes: r.sample_file_size,
                    mime: None,
                }),
                jpeg: r.jpeg_url.map(|url| ImageVariant {
                    url,
                    width: r.jpeg_width,
                    height: r.jpeg_height,
                    size_bytes: r.jpeg_file_size,
                    mime: Some("image/jpeg".into()),
                }),
            },
            extra: r.extra,
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct MoebooruRawPost {
    pub id: i64,
    pub md5: String,
    pub rating: String,
    pub score: i32,
    pub width: u32,
    pub height: u32,
    pub tags: String,

    pub file_url: String,
    pub file_size: Option<u64>,
    pub file_ext: Option<String>,

    pub jpeg_url: Option<String>,
    pub jpeg_width: Option<u32>,
    pub jpeg_height: Option<u32>,
    pub jpeg_file_size: Option<u64>,

    pub sample_url: Option<String>,
    pub sample_width: Option<u32>,
    pub sample_height: Option<u32>,
    pub sample_file_size: Option<u64>,

    pub preview_url: String,
    pub preview_width: Option<u32>,
    pub preview_height: Option<u32>,

    pub source: Option<String>,
    pub created_at: Option<i64>,
    pub author: Option<String>,

    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[async_trait]
impl ImageProvider for MoebooruProvider {
    fn id(&self) -> &str {
        &self.id
    }

    fn display_name(&self) -> &str {
        &self.display_name
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            max_results_per_page: 100,
            uses_md5: true,
            default_sort_desc_by_id: true,
        }
    }

    async fn search(&self, query: &SearchQuery, page: u32) -> Result<Vec<Post>, CoreError> {
        let tag_str = Self::build_tag_string(query);
        let limit = query
            .limit
            .min(self.capabilities().max_results_per_page)
            .max(1);
        let policy = RetryPolicy::standard();
        let raw = with_backoff(&policy, || self.fetch_page(&tag_str, page, limit)).await?;
        Ok(raw.into_iter().map(|r| self.normalize(r)).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yande_dl_core::model::Rating;

    fn dummy_query(tags: &[&str]) -> SearchQuery {
        SearchQuery {
            tags: tags.iter().map(|s| (*s).to_string()).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn build_tag_string_basic() {
        let q = dummy_query(&["stella_sora"]);
        assert_eq!(MoebooruProvider::build_tag_string(&q), "stella_sora");
    }

    #[test]
    fn build_tag_string_with_rating_subset() {
        let mut q = dummy_query(&["foo"]);
        q.ratings = vec![Rating::Safe, Rating::Questionable];
        let s = MoebooruProvider::build_tag_string(&q);
        assert!(s.contains("foo"));
        assert!(s.contains("rating:s"));
        assert!(s.contains("rating:q"));
    }

    #[test]
    fn build_tag_string_omits_rating_when_all() {
        let mut q = dummy_query(&["foo"]);
        q.ratings = vec![Rating::Safe, Rating::Questionable, Rating::Explicit];
        let s = MoebooruProvider::build_tag_string(&q);
        assert!(!s.contains("rating:"));
    }

    #[test]
    fn build_tag_string_with_score_and_size() {
        let mut q = dummy_query(&["bar"]);
        q.min_score = Some(50);
        q.min_width = Some(1920);
        q.min_height = Some(1080);
        let s = MoebooruProvider::build_tag_string(&q);
        assert!(s.contains("score:>=50"));
        assert!(s.contains("width:>=1920"));
        assert!(s.contains("height:>=1080"));
    }

    #[test]
    fn build_tag_string_empty_when_nothing() {
        let q = SearchQuery::default();
        assert_eq!(MoebooruProvider::build_tag_string(&q), "");
    }
}
