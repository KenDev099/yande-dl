use reqwest::Client;
use std::time::Duration;

pub fn build_client() -> Client {
    let ua = format!(
        "yande-dl/{} (+https://github.com/KenDev099/yande-dl)",
        env!("CARGO_PKG_VERSION")
    );
    Client::builder()
        .user_agent(ua)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(60))
        .build()
        .expect("failed to build reqwest client")
}
