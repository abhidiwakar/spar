import { useApp } from "../../lib/store";
import { detectRuntimes, loadProgress, saveSettings } from "../../lib/api";
import { useState } from "react";
import sparMark from "../../assets/spar-mark.svg";

export function SetupScreen() {
  const setScreen = useApp((s) => s.setScreen);
  const setRuntimes = useApp((s) => s.setRuntimes);
  const progress = useApp((s) => s.progress);
  const setProgress = useApp((s) => s.setProgress);
  const [pythonPath, setPythonPath] = useState(progress?.settings.pythonPath ?? "");
  const [nodePath, setNodePath] = useState(progress?.settings.nodePath ?? "");
  const [error, setError] = useState("");

  async function retry() {
    const settings = {
      pythonPath,
      nodePath,
      defaultLanguage: progress?.settings.defaultLanguage ?? "python",
      dailyGoal: progress?.settings.dailyGoal ?? 1,
      timerEnabled: progress?.settings.timerEnabled ?? true,
      openaiApiKeySet: progress?.settings.openaiApiKeySet ?? false,
      aiProvider: progress?.settings.aiProvider ?? "openai",
      ollamaHost: progress?.settings.ollamaHost ?? "http://127.0.0.1:11434",
      ollamaModel: progress?.settings.ollamaModel ?? "",
    };
    try {
      await saveSettings(settings);
      const rt = await detectRuntimes(pythonPath, nodePath);
      setRuntimes(rt);
      if (!rt.python && !rt.node) {
        setError("Still no interpreter. Install Python 3 or Node.js, or paste an absolute path.");
        return;
      }
      const snap = await loadProgress();
      setProgress(snap);
      setScreen("home");
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <div className="flex h-full items-center justify-center bg-ink-950 px-6">
      <div className="w-full max-w-lg rounded-xl border border-ink-700 bg-ink-900 p-8">
        <img src={sparMark} alt="" className="h-10 w-10 rounded-[10px]" />
        <p className="mt-4 text-xs uppercase tracking-[0.2em] text-gold-400">Spar</p>
        <h1 className="mt-2 font-serif text-3xl">Need a language runtime</h1>
        <p className="mt-3 text-sm text-paper-400">
          Spar runs your code locally. Install Python 3 and/or Node.js, or point at an existing
          binary. You only need one of them to start.
        </p>
        <label className="mt-6 block text-xs text-paper-400">Python path</label>
        <input
          className="mt-1 w-full rounded-md border border-ink-700 bg-ink-950 px-3 py-2 font-mono text-sm"
          placeholder="/usr/bin/python3"
          value={pythonPath}
          onChange={(e) => setPythonPath(e.target.value)}
        />
        <label className="mt-4 block text-xs text-paper-400">Node path</label>
        <input
          className="mt-1 w-full rounded-md border border-ink-700 bg-ink-950 px-3 py-2 font-mono text-sm"
          placeholder="/usr/local/bin/node"
          value={nodePath}
          onChange={(e) => setNodePath(e.target.value)}
        />
        {error ? <p className="mt-3 text-sm text-hard">{error}</p> : null}
        <button
          className="mt-6 rounded-md bg-gold-400 px-4 py-2 text-sm font-semibold text-ink-950"
          onClick={() => void retry()}
        >
          Detect and continue
        </button>
      </div>
    </div>
  );
}
