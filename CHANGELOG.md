# Changelog

All notable changes to this project will be documented here. The format is based
on [Keep a Changelog](https://keepachangelog.com/), and this project adheres to
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.2.0] - 2026-05-30

Drill-down browsing and bulk operations on top of the v0.1 core.

### Added

- **Tag detail page** — drill into a subscription for a paginated preview grid of
  matching posts, with multi-select, "download selected," and "download all"
  (paginate from page 1 until results run out). Includes select-all / deselect-all.
- **Display name / alias** — give a subscription a friendly label; the UI falls
  back to the tag when empty. Dedup keying and folder names still use the
  normalized tag, so aliases never affect on-disk layout.
- **Batch update** — "Update all" runs every subscription in sequence under one
  batch with live progress and a "Stop all" control; the in-progress batch
  survives a UI remount so progress is restored on return.
- **Open in browser** — open a post or a tag's listing on the source site
  (Yande.re / Konachan) directly.
- **Downloaded file count** — per-subscription count of files already on disk.

## [0.1.0] - 2026-05-08

Initial public release. The "narrow but correct" cut.

### Added

- **Subscriptions** — save a tag on Yande.re or Konachan; appears as a card
  with download / update / open-folder / remove actions.
- **Full-sweep download** for a brand-new subscription, with folder-scan dedup
  so re-runs never re-download the same post (extension-agnostic).
- **Incremental update** with a retry-aware safe baseline and a 2-page
  lookback — failed posts are not silently skipped on the next run.
- **Per-image MD5 verification** of the original variant; mismatches retry
  once under a fast policy then surface as a per-image failure.
- **Cancellation** — pressing cancel interrupts in-flight HTTP body reads
  via `tokio::select!`, not just the post queue.
- **Tag normalization & filename sanitize** — `Stella_Sora` and `stella_sora`
  collapse to one subscription; folder names are safe on Windows / macOS / Linux.
- **First-run modal** — pick download folder, default rating, age confirmation.
- **Settings** — concurrency, min request delay, default ratings, blacklist,
  theme. Plain JSON; power users can edit directly.
- **Import / Export** — share or back up subscriptions via a single
  `tags.json`, with `replace` and `merge` modes.
- **Active jobs drawer** — live progress for running downloads with a cancel
  affordance; uses bounded `mpsc` so a slow UI never backpressures the runner.
- **Polite HTTP client** — identifiable User-Agent, exponential backoff
  honoring `Retry-After`, default 3 concurrent / 300 ms delay.
- **Cross-platform CI** — GitHub Actions matrix for macOS aarch64/x86_64,
  Windows x86_64, Linux x86_64; release tags produce draft releases.

### Architecture notes

- Four-crate Rust workspace (`core / providers / config / tauri`) where
  `core` knows nothing about Tauri or JSON persistence.
- Tag normalization (`normalize_tag`) and filename sanitize
  (`safe_folder_segment`) live in core and are reused everywhere.
- `compute_safe_baseline` ensures `last_seen_post_id` never advances past a
  failure, so transient errors get retried automatically next time.
- `Downloader::scan_existing_post_ids` reaps stale `.part` files older than
  24 h while building the dedup set.

### Known limitations

- macOS / Windows binaries are unsigned — first-run requires user approval.
  Code signing arrives in v0.2.
- Single-tag subscriptions only. Multi-tag (AND / OR / NOT) lands in v0.2.
- Always downloads the original variant. JPG-mode (without MD5 verification)
  is planned for v0.2.

[Unreleased]: https://github.com/KenDev099/yande-dl/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/KenDev099/yande-dl/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/KenDev099/yande-dl/releases/tag/v0.1.0
