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
