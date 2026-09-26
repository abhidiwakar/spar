# Spar

A local desktop gym for data structures and algorithms. LeetCode-style workspace, Python, JavaScript, and C++, a 12-week path, streaks and XP. No hearts — you can fail Submit as often as you want.

Not affiliated with LeetCode or any employer.

## Warning

**Run and Submit still execute your code on this machine.** The judge now kills the runner after a time limit, a 512 MB memory cap, and a process cap, and on macOS it applies a Seatbelt profile (no network, no writes outside the temp workdir, no extra executables). That is isolation for runaway solutions, not a security boundary. Only run code you wrote or fully trust.

## Prerequisites

- Node.js 18+
- Python 3 (to run Python solutions)
- A C++17 compiler such as `g++` or `clang++` (to run C++ solutions)
- Rust (stable) — [rustup](https://rustup.rs)
- Xcode Command Line Tools on macOS; see [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) on Linux or Windows

## Run

```bash
npm install
npm run tauri dev
```

Problem JSON under `content/problems/` is already generated. Re-run `python3 scripts/gen_problems.py` only if you change the generator.

First launch: Home → **Continue** → Two Sum. Pick Python, JavaScript, or C++, **Run** visible tests, **Submit** hidden tests.

If the app cannot find `python3`, `node`, or a C++ compiler, it opens setup so you can paste an absolute path.

## Downloads

Pushes to `main` run tests first. If they pass, the patch version is bumped and installers are published on [Releases](https://github.com/abhidiwakar/spar/releases): a signed, notarized Apple Silicon `.dmg` and Windows `.exe` / `.msi`. To cut `0.2.0` instead of `0.1.x`, set that version in `src-tauri/tauri.conf.json` before you push. Pull requests run tests only.

The Microsoft Store listing is an **MSIX** upload (Microsoft signs it). See [docs/msix-store.md](docs/msix-store.md). macOS signing needs the secrets in [docs/macos-signing.md](docs/macos-signing.md). Windows SmartScreen may still warn on the GitHub `.exe` / `.msi`.

## Tests

```bash
npx tsc --noEmit
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

## Layout

- `content/` — units and problems (classic algorithms, original statements)
- `src/` — React workspace (Home, Path, editor, Progress, Settings)
- `src-tauri/` — judge (`python3` / `node` / `g++`) and SQLite progress

Progress lives in the app data directory. Optional AI review uses an OpenAI key or a local Ollama model; the key stays in SQLite and is never sent to the UI.

Hidden tests and editorials live in `content/` for authoring. The renderer only sees visible cases. Packaged apps download `content/` from this repo's `main` branch on first launch (Setting up), then cache it under the app data directory and refresh in the background on later launches. Editorials load after Accepted or a hint.

## License

[MIT](LICENSE)
