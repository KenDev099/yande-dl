use std::path::Path;

fn main() {
    // generate_context!() requires `frontendDist` (ui/dist) to exist when the
    // crate is compiled, even in dev mode (where the webview actually loads
    // devUrl). Stamp a placeholder so a fresh clone can `cargo build` before
    // the frontend has ever been built.
    let dist = Path::new("../../ui/dist");
    if !dist.exists() {
        let _ = std::fs::create_dir_all(dist);
    }
    let index = dist.join("index.html");
    if !index.exists() {
        let _ = std::fs::write(
            &index,
            "<!doctype html><html><head><meta charset=\"UTF-8\"></head><body><div id=\"root\"></div></body></html>",
        );
    }

    tauri_build::build()
}
