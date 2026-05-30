<div align="center">
  <img src="docs/logo.svg" width="140" alt="yande-dl logo" />

  <h1>yande-dl</h1>

  <p>
    A modern, lean image board subscription downloader.<br/>
    No database. No lock-in. Just folders.
  </p>

  <p>
    <strong>English</strong> ·
    <a href="README.zh-TW.md">繁體中文</a> ·
    <a href="README.zh-CN.md">简体中文</a>
  </p>

  <p>
    <a href="https://github.com/KenDev099/yande-dl/releases/latest"><img src="https://img.shields.io/github/v/release/KenDev099/yande-dl?sort=semver" alt="Latest release" /></a>
    <a href="https://github.com/KenDev099/yande-dl/releases"><img src="https://img.shields.io/github/downloads/KenDev099/yande-dl/total" alt="Total downloads" /></a>
  </p>
</div>

---

> Most booru downloaders either feel like Windows 95 or want to be your library manager.
> yande-dl is neither — it does three things: subscribe to tags, batch-download every
> matching post, and stay out of your way. Your files live in folders you choose,
> browseable in any file manager.

## Features

- **Tag-based bulk download** — fetch every post for a tag, paginated, deduplicated by filename.
- **Incremental updates with retry-aware baselines** — re-run a saved tag, only download what's new. Failed images are retried automatically next time, never silently skipped.
- **Multi-platform support** — Yande.re and Konachan; architecture ready for more.
- **Multi-language UI** — English, 繁體中文, 简体中文. Auto-detects your OS locale; switchable in Settings.
- **Import/Export** — your subscriptions are a single `tags.json` file.
- **Polite by design** — conservative concurrency (default 3), 300 ms minimum delay, identifiable User-Agent, default safe rating.
- **Modern UI** — Tauri 2 + React, dark by default.
- **Local-first, zero-DB** — JSON config + folder scan, no SQLite, no telemetry.

## Installation

Download a build from the [latest release](https://github.com/KenDev099/yande-dl/releases/latest):

- macOS: `yande-dl_<ver>_aarch64.dmg` / `_x64.dmg`
- Windows: `yande-dl_<ver>_x64-setup.exe`
- Linux: `yande-dl_<ver>_amd64.deb` / `.AppImage`

> **Unsigned binaries** — code signing arrives in a later release. First launch:
> on macOS, right-click the app → **Open** → confirm (or run
> `xattr -dr com.apple.quarantine /Applications/yande-dl.app`); on Windows,
> SmartScreen → **More info** → **Run anyway**.

### Build from source

```bash
git clone https://github.com/KenDev099/yande-dl
cd yande-dl

# Frontend deps + Rust toolchain prerequisites for Tauri 2 (see https://tauri.app/start/prerequisites/).
pnpm install --dir ui

# Run from the workspace root — the Tauri CLI needs to see crates/yande-dl-tauri/tauri.conf.json.
pnpm dev          # tauri dev (live-reload Rust + Vite)
pnpm build        # tauri build (production bundle)
```

Requirements: Rust 1.75+, Node 20+, pnpm 9+, Tauri prerequisites for your platform.

## Usage

1. Launch and complete the first-run modal (download folder, default rating, age confirmation).
2. Add a tag from **Subscriptions** (e.g., `stella_sora` on Yande.re).
3. Click **Download** — yande-dl saves to `<root>/_yande stella_sora/yande_<post_id>.<ext>`.
4. Browse the folder in your OS file manager. yande-dl is intentionally not a viewer.

After the first run, **Update** fetches only posts newer than the last run, plus a
2-page lookback to recover any image that transiently failed earlier.

## Configuration

Settings live in the app, stored as plain JSON:

- macOS: `~/Library/Application Support/yande-dl/`
- Windows: `%APPDATA%\yande-dl\`
- Linux: `~/.config/yande-dl/`

Power users can edit `tags.json` and `settings.json` directly. Atomic writes
ensure consistency. To opt-in to verbose logs, set `KURA_LOG=debug`.

## Architecture (one paragraph)

The Rust core has four crates: **`yande-dl-core`** owns the data model,
`ImageProvider` trait, sanitize and retry helpers, the `Downloader` (folder-scan
dedup, MD5-verified, cancellable), and the `JobRunner` (incremental lookback,
retry-aware safe baseline). **`yande-dl-providers`** implements `MoebooruProvider`
shared between Yande.re and Konachan. **`yande-dl-config`** persists `tags.json`
and `settings.json` with atomic writes and corruption recovery. **`yande-dl-tauri`**
is the shell — Tauri 2 + commands + events. The frontend is React 18 + Tailwind +
shadcn-style components driven by TanStack Query and a typed IPC layer.

## ⚖️ Legal Notice & Responsible Use

yande-dl is a **client tool**. It does not host, distribute, or generate any image content.
All images are fetched directly from third-party services to the user's local machine.

**Users are solely responsible for:**

- Confirming they meet the legal age requirement in their jurisdiction.
- Complying with the terms of service of the source websites.
- Respecting the copyright of individual images (most posts are user-uploaded fan art;
  original artists retain copyright — please support them directly when you can).

**yande-dl is designed to be a polite client:**

- Conservative rate limiting (3 concurrent, 300 ms minimum interval by default).
- Honors `Retry-After` headers.
- Identifiable User-Agent: `yande-dl/<version> (+https://github.com/KenDev099/yande-dl)`.
- Default `safe` rating filter; explicit opt-in is required for adult content.

## Roadmap

- [x] **v0.1.0** — Yande.re + Konachan, subscriptions, incremental updates, import/export, i18n
- [ ] **v0.2** — Multi-tag search, "preview before download", JPG mode, command palette, auto-update
- [ ] **v1.0** — Danbooru, Gelbooru, e621, pools, CLI mode

## License

[MIT](LICENSE) © 2026

## Acknowledgements

- [Yande.re](https://yande.re), [Konachan](https://konachan.com), and the broader Moebooru community.
- [Tauri](https://tauri.app), [shadcn/ui](https://ui.shadcn.com), [TanStack](https://tanstack.com), [Radix UI](https://radix-ui.com).
- All artists whose work graces the boards.
