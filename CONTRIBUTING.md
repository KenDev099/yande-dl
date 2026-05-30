# Contributing to yande-dl

Thanks for your interest! yande-dl is a small project with a narrow scope —
"subscribe, download, stay out of the way". Contributions are welcome,
particularly bug fixes, additional providers, and platform polish.

## Dev setup

```bash
git clone https://github.com/KenDev099/yande-dl
cd yande-dl

# Frontend deps (also brings the Tauri CLI as a devDependency)
pnpm install --dir ui

# Run dev — must be invoked from the workspace root so the Tauri CLI
# can locate crates/yande-dl-tauri/tauri.conf.json.
pnpm dev
```

Required toolchain:

- Rust **1.75+** (stable)
- Node **20+** and **pnpm 9+**
- Tauri 2 platform prerequisites — see <https://tauri.app/start/prerequisites/>

## Workflow

```bash
# Rust
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all

# Frontend
pnpm --dir ui typecheck
pnpm --dir ui build
```

CI runs all of the above. Please confirm locally before opening a PR.

## Project layout

```
crates/
  yande-dl-core/       # data model, ImageProvider trait, downloader, job runner
  yande-dl-providers/  # MoebooruProvider (Yande.re, Konachan)
  yande-dl-config/     # tags.json, settings.json (atomic JSON)
  yande-dl-tauri/      # Tauri shell + commands + events
ui/                    # React 18 + Vite + Tailwind frontend
docs/spec-v6.md        # design spec — read this before adding features
```

## Pull request guidelines

- Keep PRs focused. Bug fix + refactor + feature in one PR is hard to review.
- Match the existing style — if you'd "do it differently", say so in the PR
  body, but don't refactor adjacent unrelated code.
- Add tests for any non-trivial change to `yande-dl-core` or `yande-dl-config`.
- Update `docs/spec-v6.md` if you change a documented invariant.
- Use Conventional-style messages where natural (`feat:`, `fix:`, `chore:`).

## Releasing

Every push to `main` runs `.github/workflows/release.yml`, which reads the
version from `crates/yande-dl-tauri/tauri.conf.json`, builds for macOS
arm64/x64, Linux x64, and Windows x64, and publishes a GitHub Release tagged
`v<version>` with all artifacts attached. To cut a new release:

```bash
pnpm bump 0.2.0                                      # syncs 4 version files
# update CHANGELOG.md
git add -A && git commit -m "chore: release v0.2.0"
git push                                             # main -> builds + publishes
```

`pnpm bump` writes the new version into `Cargo.toml` (workspace), root
`package.json`, `ui/package.json`, and `crates/yande-dl-tauri/tauri.conf.json`.
All crates inherit via `version.workspace = true`. The workflow creates the
`v<version>` tag for you — no manual `git tag` needed.

Versions containing `-` (e.g. `0.2.0-beta.1`) are auto-marked as prereleases.

The release is created as a draft and is flipped to public only after all four
platform builds succeed, so external users never see a half-built release. If a
platform fails, fix it and re-run — the release stays a draft until complete.

Pushes that keep the same version refresh that release's binaries in place (the
`v<version>` tag stays at the commit where the version was first created); bump
the version for a clean, separately-tagged release.

## Adding a new provider

1. Implement `ImageProvider` in `crates/yande-dl-providers/src/<name>.rs`.
2. Re-export from the providers crate's `lib.rs`.
3. Register the provider in `crates/yande-dl-tauri/src/setup.rs`.
4. Add the provider to the frontend dropdown in
   `ui/src/components/AddSubscriptionDialog.tsx`.
5. Add the post-URL template to `crates/yande-dl-tauri/src/commands/system.rs::open_post_url`.
6. Add wiremock-driven tests with a real-API fixture under
   `crates/yande-dl-providers/tests/fixtures/`.

## Reporting bugs

Open an issue with:

- yande-dl version (`KURA_LOG=debug` log fragment if relevant)
- OS + version
- Steps to reproduce
- Whether the offending tag is reproducible publicly (don't share a private one)

## License

By contributing, you agree your code is licensed under the
[MIT License](LICENSE).
