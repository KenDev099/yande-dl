//! yande-dl-core: provider-agnostic abstractions and the download engine.
//!
//! This crate has no knowledge of Tauri, JSON persistence, or specific image boards.
//! It exposes the [`provider::ImageProvider`] trait, the data model, sanitize
//! helpers, the retry helper, the [`downloader::Downloader`], and the
//! [`job::run_job`] task runner.

pub mod downloader;
pub mod error;
pub mod job;
pub mod model;
pub mod provider;
pub mod retry;
pub mod sanitize;
