#!/usr/bin/env node
// Syncs the project version across the 4 places it lives:
//   - Cargo.toml                                (workspace.package.version — all crates inherit)
//   - package.json                              (root)
//   - ui/package.json
//   - crates/yande-dl-tauri/tauri.conf.json
//
// Usage: node scripts/bump-version.mjs <semver>
//   e.g. node scripts/bump-version.mjs 0.2.0
//        node scripts/bump-version.mjs 0.2.0-beta.1

import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const next = process.argv[2];

if (!next) {
  console.error("error: missing version argument");
  console.error("usage: node scripts/bump-version.mjs <semver>");
  process.exit(1);
}

if (!/^\d+\.\d+\.\d+(?:-[\w.]+)?$/.test(next)) {
  console.error(`error: "${next}" is not a valid semver (expected X.Y.Z or X.Y.Z-tag)`);
  process.exit(1);
}

const targets = [
  { path: "Cargo.toml", kind: "toml-workspace" },
  { path: "package.json", kind: "json" },
  { path: "ui/package.json", kind: "json" },
  { path: "crates/yande-dl-tauri/tauri.conf.json", kind: "json" },
];

let previous = null;
const summary = [];

for (const t of targets) {
  const abs = join(root, t.path);
  const raw = readFileSync(abs, "utf8");
  let updated;
  let found;

  if (t.kind === "json") {
    const obj = JSON.parse(raw);
    found = obj.version;
    obj.version = next;
    updated = JSON.stringify(obj, null, 2) + (raw.endsWith("\n") ? "\n" : "");
  } else if (t.kind === "toml-workspace") {
    // Match the first `version = "..."` line inside [workspace.package].
    // Why: editing TOML with regex is brittle in general, but our Cargo.toml
    // has exactly one workspace.package version line and no other top-level
    // version fields, so this is safe.
    const re = /(\[workspace\.package\][^\[]*?\bversion\s*=\s*")([^"]+)(")/s;
    const m = raw.match(re);
    if (!m) throw new Error(`could not find [workspace.package] version in ${t.path}`);
    found = m[2];
    updated = raw.replace(re, `$1${next}$3`);
  }

  if (previous === null) previous = found;
  else if (previous !== found) {
    console.warn(`warn: ${t.path} had version "${found}", expected "${previous}" (drift detected, syncing anyway)`);
  }

  writeFileSync(abs, updated);
  summary.push(`  ${t.path}: ${found} → ${next}`);
}

console.log(`Bumped ${previous} → ${next} (${targets.length} files)`);
for (const line of summary) console.log(line);
console.log("\nNext steps:");
console.log("  update CHANGELOG.md");
console.log(`  git add -A && git commit -m "chore: release v${next}"`);
console.log("  git push   # pushing main builds every platform and publishes the v" + next + " release");
