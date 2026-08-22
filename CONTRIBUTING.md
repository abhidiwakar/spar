# Contributing

## Develop

```bash
npm install
npm run tauri dev
```

Need a language runtime on the PATH (`python3` and/or `node`), plus Rust.

## Checks

```bash
npx tsc --noEmit
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

Run both before you open a pull request.

## Problems

Authoring source of truth is `content/problems/*.json` (or `scripts/gen_problems.py`). Hidden tests and editorials stay in those files; the Vite build strips them from the renderer and the packaged app embeds the catalog in the binary.

Keep problem statements as original write-ups of classic algorithms. Do not paste third-party premium or copyrighted problem text.

## Pull requests

Small, reviewable diffs. Describe why the change exists. Be specific in issue reports (OS, Rust/Node versions, what you expected).
