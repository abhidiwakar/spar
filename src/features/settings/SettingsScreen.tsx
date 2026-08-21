import { type ReactNode, useEffect, useState } from "react";
import { detectRuntimes, loadProgress, saveSettings } from "../../lib/api";
import { useApp } from "../../lib/store";
import type { Language, Runtimes } from "../../lib/types";

export function SettingsScreen() {
  const progress = useApp((s) => s.progress);
  const setProgress = useApp((s) => s.setProgress);
  const setRuntimes = useApp((s) => s.setRuntimes);
  const setLanguage = useApp((s) => s.setLanguage);
  const runtimes = useApp((s) => s.runtimes);
  const [pythonPath, setPythonPath] = useState(progress?.settings.pythonPath ?? "");
  const [nodePath, setNodePath] = useState(progress?.settings.nodePath ?? "");
  const [defaultLanguage, setDefaultLanguage] = useState(progress?.settings.defaultLanguage ?? "python");
  const [dailyGoal, setDailyGoal] = useState(progress?.settings.dailyGoal ?? 1);
  const [timerEnabled, setTimerEnabled] = useState(progress?.settings.timerEnabled ?? true);
  const [openaiApiKey, setOpenaiApiKey] = useState(progress?.settings.openaiApiKey ?? "");
  const [msg, setMsg] = useState("");
  const [detected, setDetected] = useState<Runtimes | null>(runtimes);

  useEffect(() => {
    if (!progress) return;
    setPythonPath(progress.settings.pythonPath);
    setNodePath(progress.settings.nodePath);
    setDefaultLanguage(progress.settings.defaultLanguage);
    setDailyGoal(progress.settings.dailyGoal);
    setTimerEnabled(progress.settings.timerEnabled);
    setOpenaiApiKey(progress.settings.openaiApiKey ?? "");
  }, [progress]);

  useEffect(() => {
    let cancelled = false;
    void detectRuntimes().then((rt) => {
      if (!cancelled) setDetected(rt);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  async function save() {
    await saveSettings({
      pythonPath,
      nodePath,
      defaultLanguage,
      dailyGoal,
      timerEnabled,
      openaiApiKey,
    });
    const rt = await detectRuntimes(pythonPath, nodePath);
    setRuntimes(rt);
    const auto = await detectRuntimes();
    setDetected(auto);
    setLanguage((defaultLanguage as Language) || "python");
    setProgress(await loadProgress());
    setMsg("Saved.");
  }

  return (
    <div className="mx-auto max-w-xl px-6 py-10">
      <h1 className="font-serif text-3xl">Settings</h1>
      <div className="mt-8 space-y-5">
        <Field label="Python path">
          <input
            className="w-full rounded-md border border-ink-700 bg-ink-900 px-3 py-2 font-mono text-sm"
            value={pythonPath}
            onChange={(e) => setPythonPath(e.target.value)}
          />
          <PathHint
            override={pythonPath}
            detected={detected?.python}
            ready={detected !== null}
          />
        </Field>
        <Field label="Node path">
          <input
            className="w-full rounded-md border border-ink-700 bg-ink-900 px-3 py-2 font-mono text-sm"
            value={nodePath}
            onChange={(e) => setNodePath(e.target.value)}
          />
          <PathHint
            override={nodePath}
            detected={detected?.node}
            ready={detected !== null}
          />
        </Field>
        <Field label="Default language">
          <select
            className="w-full rounded-md border border-ink-700 bg-ink-900 px-3 py-2 text-sm"
            value={defaultLanguage}
            onChange={(e) => setDefaultLanguage(e.target.value)}
          >
            <option value="python">Python</option>
            <option value="javascript">JavaScript</option>
          </select>
        </Field>
        <Field label="Daily accepted goal">
          <input
            type="number"
            min={1}
            className="w-full rounded-md border border-ink-700 bg-ink-900 px-3 py-2 text-sm"
            value={dailyGoal}
            onChange={(e) => setDailyGoal(Number(e.target.value) || 1)}
          />
        </Field>
        <label className="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            checked={timerEnabled}
            onChange={(e) => setTimerEnabled(e.target.checked)}
          />
          Show interview timer on problems
        </label>
        <Field label="OpenAI API key">
          <input
            type="password"
            autoComplete="off"
            className="w-full rounded-md border border-ink-700 bg-ink-900 px-3 py-2 font-mono text-sm"
            value={openaiApiKey}
            onChange={(e) => setOpenaiApiKey(e.target.value)}
            placeholder="sk-…"
          />
          <p className="mt-1 text-xs text-paper-500">
            Stored only on this machine. Used after Accepted to critique your solution — not to write
            it for you.
          </p>
        </Field>
        <button
          className="rounded-md bg-gold-400 px-4 py-2 text-sm font-semibold text-ink-950"
          onClick={() => void save()}
        >
          Save
        </button>
        {msg ? <p className="text-sm text-easy">{msg}</p> : null}
      </div>
    </div>
  );
}

function PathHint({
  override,
  detected,
  ready,
}: {
  override: string;
  detected?: string | null;
  ready: boolean;
}) {
  if (override.trim() || !ready) return null;
  if (detected) {
    return (
      <p className="mt-1 font-mono text-xs text-paper-500">Using auto-detected {detected}</p>
    );
  }
  return (
    <p className="mt-1 text-xs text-paper-500">
      Not found on PATH. Paste an absolute path or install the runtime.
    </p>
  );
}

function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label className="block">
      <span className="text-xs text-paper-400">{label}</span>
      <div className="mt-1">{children}</div>
    </label>
  );
}
