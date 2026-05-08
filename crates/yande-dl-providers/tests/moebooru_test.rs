use reqwest::Client;
use std::time::Duration;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};
use yande_dl_core::error::CoreError;
use yande_dl_core::model::{Rating, SearchQuery};
use yande_dl_core::provider::ImageProvider;
use yande_dl_providers::MoebooruProvider;

const FIXTURE: &str = include_str!("fixtures/yande_post_response.json");

fn build_provider(base_url: String) -> MoebooruProvider {
    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    MoebooruProvider::with_base_url("yande", "Yande.re", base_url, client)
}

fn query() -> SearchQuery {
    SearchQuery {
        tags: vec!["stella_sora".into()],
        limit: 100,
        ..Default::default()
    }
}

#[tokio::test]
async fn search_parses_normal_response() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/post.json"))
        .and(query_param("tags", "stella_sora"))
        .and(query_param("page", "1"))
        .and(query_param("limit", "100"))
        .respond_with(ResponseTemplate::new(200).set_body_string(FIXTURE))
        .mount(&server)
        .await;

    let p = build_provider(server.uri());
    let posts = p.search(&query(), 1).await.expect("search ok");

    assert_eq!(posts.len(), 3);

    let p0 = &posts[0];
    assert_eq!(p0.provider_id, "yande");
    assert_eq!(p0.post_id, 1255110);
    assert_eq!(p0.md5, "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4");
    assert_eq!(p0.rating, Rating::Safe);
    assert_eq!(p0.score, 87);
    assert_eq!(p0.width, 2400);
    assert_eq!(p0.height, 3600);
    assert!(p0.tags.contains(&"stella_sora".to_string()));
    assert!(p0.tags.contains(&"flowers".to_string()));
    assert_eq!(p0.artist.as_deref(), Some("ExampleArtist"));
    assert_eq!(
        p0.source_url.as_deref(),
        Some("https://www.pixiv.net/artworks/123456")
    );
    assert_eq!(p0.variants.original.size_bytes, Some(4521374));
    assert_eq!(p0.variants.original.mime.as_deref(), Some("image/png"));
    assert!(p0.variants.jpeg.is_some());
    assert!(p0.variants.sample.is_some());
}

#[tokio::test]
async fn search_handles_missing_jpeg_and_sample() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/post.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(FIXTURE))
        .mount(&server)
        .await;

    let p = build_provider(server.uri());
    let posts = p.search(&query(), 1).await.expect("search ok");

    let p1 = &posts[1];
    assert_eq!(p1.post_id, 1255111);
    assert_eq!(p1.rating, Rating::Questionable);
    assert!(p1.variants.jpeg.is_none(), "jpeg should be None");
    assert!(p1.variants.sample.is_none(), "sample should be None");
    assert!(p1.source_url.is_none());
    assert_eq!(p1.variants.original.mime.as_deref(), Some("image/jpg"));
}

#[tokio::test]
async fn search_retries_on_429_with_retry_after() {
    let server = MockServer::start().await;

    // First call: 429 with Retry-After. Second call: 200.
    Mock::given(method("GET"))
        .and(path("/post.json"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "0")
                .set_body_string("rate limited"),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/post.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(FIXTURE))
        .mount(&server)
        .await;

    let p = build_provider(server.uri());
    let posts = p.search(&query(), 1).await.expect("eventually ok");
    assert_eq!(posts.len(), 3);
}

#[tokio::test]
async fn search_gives_up_after_repeated_5xx() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/post.json"))
        .respond_with(ResponseTemplate::new(500).set_body_string("oops"))
        .mount(&server)
        .await;

    // Patch policy by reducing client timeout so test is quick — but the
    // backoff itself is 2s/4s; standard policy will take ~6s in worst case.
    // With max_attempts=3 and base 2000ms it's 0 + 2 + 4 = 6s total. To keep
    // the test fast, we accept this and rely on tokio's pause/auto-advance
    // when feasible. For now we just let it run.
    let p = build_provider(server.uri());
    let res = p.search(&query(), 1).await;
    assert!(matches!(res, Err(CoreError::Server { status: 500 })));
}
