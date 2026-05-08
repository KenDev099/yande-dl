use crate::error::CoreError;
use crate::model::{Capabilities, Post, SearchQuery};
use async_trait::async_trait;

#[async_trait]
pub trait ImageProvider: Send + Sync {
    fn id(&self) -> &str;
    fn display_name(&self) -> &str;
    fn capabilities(&self) -> Capabilities;

    /// Fetch a single page of search results. Pages start from 1; an empty
    /// vector means "no more results".
    async fn search(&self, query: &SearchQuery, page: u32) -> Result<Vec<Post>, CoreError>;
}
