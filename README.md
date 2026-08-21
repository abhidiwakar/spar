# Spar

Desktop gym for Microsoft-style DSA interviews. LeetCode layout, Python and JavaScript, a 12-week path, streaks and XP. No hearts — you can fail Submit as often as you want.

## Prerequisites

- Node.js 18+
- Python 3 (to run Python solutions)
- Rust (stable) — [rustup](https://rustup.rs) or `brew install rust`
- Xcode Command Line Tools on macOS

## Run

```bash
npm install
python3 scripts/gen_problems.py   # already run if content/problems exists
npm run tauri dev
```

First launch: Home → **Continue** → Two Sum. Pick Python or JavaScript, **Run** visible tests, **Submit** hidden tests.

If the app cannot find `python3` or `node`, it opens setup so you can paste an absolute path.

## Layout

- `content/` — units and problems (original statements, classic algorithms)
- `src/` — React workspace (Home, Path, editor, Progress, Settings)
- `src-tauri/` — judge (`python3` / `node`) and SQLite progress

Progress is stored locally in the app data directory.
