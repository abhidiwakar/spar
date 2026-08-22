# Spar

A local desktop gym for data structures and algorithms. LeetCode-style workspace, Python and JavaScript, a 12-week path, streaks and XP. No hearts — you can fail Submit as often as you want.

Not affiliated with LeetCode or any employer.

## Warning

**Run and Submit execute your code on this machine with your user account.** There is no sandbox. Only run code you wrote or fully trust.

## Prerequisites

- Node.js 18+
- Python 3 (to run Python solutions)
- Rust (stable) — [rustup](https://rustup.rs)
- Xcode Command Line Tools on macOS; see [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) on Linux or Windows

## Run

```bash
npm install
npm run tauri dev
```

Problem JSON under `content/problems/` is already generated. Re-run `python3 scripts/gen_problems.py` only if you change the generator.

First launch: Home → **Continue** → Two Sum. Pick Python or JavaScript, **Run** visible tests, **Submit** hidden tests.

If the app cannot find `python3` or `node`, it opens setup so you can paste an absolute path.

## Downloads

Pushes to `main` bump the patch version and publish an Apple Silicon `.dmg` on [Releases](https://github.com/abhidiwakar/spar/releases). To cut `0.2.0` instead of `0.1.x`, set that version in `src-tauri/tauri.conf.json` before you push.

Gatekeeper will warn because the build is not Apple-notarized. Open via right-click → Open.

## Tests

```bash
npx tsc --noEmit
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

## Layout

- `content/` — units and problems (classic algorithms, original statements)
- `src/` — React workspace (Home, Path, editor, Progress, Settings)
- `src-tauri/` — judge (`python3` / `node`) and SQLite progress

Progress lives in the app data directory. Optional AI review uses an OpenAI key or a local Ollama model; the key stays in SQLite and is never sent to the UI.

Hidden tests and editorials live in `content/` for authoring. The renderer only sees visible cases. The packaged app compiles the catalog into the binary instead of copying `content/` into Resources. Editorials load after Accepted or a hint.

## License

[MIT](LICENSE)
