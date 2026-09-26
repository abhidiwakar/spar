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

Authoring source of truth is `content/problems/*.json` (or `scripts/gen_problems.py`). Hidden tests and editorials stay in those files. The Rust side strips them before the renderer sees the catalog. Packaged builds do not embed problems: the first launch downloads `content/` from GitHub `main` onto a Setting up screen, later launches open the app-data cache and refresh in the background. `npm run tauri dev` still reads the local `content/` directory.

Keep problem statements as original write-ups of classic algorithms. Do not paste third-party premium or copyrighted problem text.

## Pull requests

Small, reviewable diffs. Describe why the change exists. Be specific in issue reports (OS, Rust/Node versions, what you expected).
