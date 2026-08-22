import { type ReactNode, useCallback, useEffect, useState } from "react";
import {
  clearOpenaiKey,
  detectRuntimes,
  loadProgress,
  ollamaCancelPull,
  ollamaPullModel,
  ollamaStatus,
  openExternalUrl,
  saveOllamaSelection,
  saveSettings,
} from "../../lib/api";
import { useApp } from "../../lib/store";
import type { AiProvider, Language, Runtimes } from "../../lib/types";

const RECOMMENDED_MODELS = [
  { id: "qwen2.5-coder:7b", label: "qwen2.5-coder:7b (recommended)" },
  { id: "qwen2.5-coder:3b", label: "qwen2.5-coder:3b (smaller)" },
] as const;

const DEFAULT_OLLAMA_HOST = "http://127.0.0.1:11434";

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(1)} MB`;
  return `${(n / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

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
  const [openaiApiKey, setOpenaiApiKey] = useState("");
  const [aiProvider, setAiProvider] = useState<AiProvider>(
    (progress?.settings.aiProvider as AiProvider) === "ollama" ? "ollama" : "openai",
  );
  const [ollamaHost, setOllamaHost] = useState(
    progress?.settings.ollamaHost || DEFAULT_OLLAMA_HOST,
  );
  const [ollamaModel, setOllamaModel] = useState(progress?.settings.ollamaModel ?? "");
  const [msg, setMsg] = useState("");
  const [detected, setDetected] = useState<Runtimes | null>(runtimes);
  const [ollamaRunning, setOllamaRunning] = useState(false);
  const [installedModels, setInstalledModels] = useState<string[]>([]);
  const [ollamaChecking, setOllamaChecking] = useState(false);
  const [pulling, setPulling] = useState<string | null>(null);
  const [pullStatus, setPullStatus] = useState("");
  const [pullCompleted, setPullCompleted] = useState<number | null>(null);
  const [pullTotal, setPullTotal] = useState<number | null>(null);
  const [ollamaError, setOllamaError] = useState("");

  useEffect(() => {
    if (!progress) return;
    setPythonPath(progress.settings.pythonPath);
    setNodePath(progress.settings.nodePath);
    setDefaultLanguage(progress.settings.defaultLanguage);
    setDailyGoal(progress.settings.dailyGoal);
    setTimerEnabled(progress.settings.timerEnabled);
    setAiProvider(progress.settings.aiProvider === "ollama" ? "ollama" : "openai");
    setOllamaHost(progress.settings.ollamaHost || DEFAULT_OLLAMA_HOST);
    setOllamaModel(progress.settings.ollamaModel ?? "");
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

  const settingsPayload = useCallback(
    (model = ollamaModel) => ({
      pythonPath,
      nodePath,
      defaultLanguage,
      dailyGoal,
      timerEnabled,
      openaiApiKey,
      openaiApiKeySet: progress?.settings.openaiApiKeySet ?? false,
      aiProvider,
      ollamaHost: ollamaHost.trim() || DEFAULT_OLLAMA_HOST,
      ollamaModel: model,
    }),
    [
      pythonPath,
      nodePath,
      defaultLanguage,
      dailyGoal,
      timerEnabled,
      openaiApiKey,
      progress?.settings.openaiApiKeySet,
      aiProvider,
      ollamaHost,
      ollamaModel,
    ],
  );

  const persistSettings = useCallback(
    async (model = ollamaModel) => {
      await saveSettings(settingsPayload(model));
      setProgress(await loadProgress());
    },
    [ollamaModel, setProgress, settingsPayload],
  );

  const refreshOllama = useCallback(
    async (host: string) => {
      setOllamaChecking(true);
      setOllamaError("");
      try {
        const status = await ollamaStatus(host);
        setOllamaRunning(status.running);
        setInstalledModels(status.models);
        if (status.running && status.models.length > 0) {
          let selected = "";
          setOllamaModel((current) => {
            if (current && status.models.includes(current)) {
              selected = current;
              return current;
            }
            const preferred = RECOMMENDED_MODELS.find((m) => status.models.includes(m.id));
            selected = preferred?.id ?? status.models[0] ?? "";
            return selected;
          });
          if (selected && selected !== ollamaModel) {
            await saveOllamaSelection({
              provider: "ollama",
              host: host.trim() || DEFAULT_OLLAMA_HOST,
              model: selected,
            });
            setProgress(await loadProgress());
          }
        }
      } catch (e) {
        setOllamaRunning(false);
        setInstalledModels([]);
        setOllamaError(String(e));
      } finally {
        setOllamaChecking(false);
      }
    },
    [ollamaModel, setProgress],
  );

  useEffect(() => {
    if (aiProvider !== "ollama") return;
    const id = window.setTimeout(() => {
      void refreshOllama(ollamaHost);
    }, 400);
    return () => window.clearTimeout(id);
  }, [aiProvider, ollamaHost, refreshOllama]);

  async function save() {
    try {
      await persistSettings();
      setOpenaiApiKey("");
      const rt = await detectRuntimes(pythonPath, nodePath);
      setRuntimes(rt);
      const auto = await detectRuntimes();
      setDetected(auto);
      setLanguage((defaultLanguage as Language) || "python");
      setMsg("Saved.");
    } catch (e) {
      setMsg("");
      setOllamaError(String(e));
    }
  }

  function resetPullUi() {
    setPulling(null);
    setPullStatus("");
    setPullCompleted(null);
    setPullTotal(null);
  }

  async function pullModel(model: string) {
    setPulling(model);
    setPullStatus("Starting…");
    setPullCompleted(null);
    setPullTotal(null);
    setOllamaError("");
    try {
      await ollamaPullModel(ollamaHost, model, (progress) => {
        setPullStatus(progress.status || "Downloading…");
        if (progress.completed != null) setPullCompleted(progress.completed);
        if (progress.total != null && progress.total > 0) setPullTotal(progress.total);
      });
      setOllamaModel(model);
      await saveOllamaSelection({
        provider: "ollama",
        host: ollamaHost.trim() || DEFAULT_OLLAMA_HOST,
        model,
      });
      setProgress(await loadProgress());
      await refreshOllama(ollamaHost);
      setPullStatus("Installed.");
      setPullCompleted(null);
      setPullTotal(null);
      setPulling(null);
    } catch (e) {
      const message = String(e);
      resetPullUi();
      if (/pull cancelled/i.test(message)) {
        setOllamaError("");
        setPullStatus("Download cancelled.");
        return;
      }
      setOllamaError(message);
    }
  }

  async function cancelPull() {
    await ollamaCancelPull();
  }

  const modelInstalled = (name: string) => installedModels.includes(name);
  const pullPct =
    pullCompleted != null && pullTotal != null && pullTotal > 0
      ? Math.min(100, Math.round((pullCompleted / pullTotal) * 100))
      : null;
  const pullRemaining =
    pullCompleted != null && pullTotal != null && pullTotal > pullCompleted
      ? pullTotal - pullCompleted
      : null;

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

        <div className="rounded-lg border border-ink-700 bg-ink-900/50 p-4">
          <p className="text-xs uppercase tracking-wide text-paper-400">AI review</p>
          <p className="mt-1 text-xs text-paper-500">
            Used after Accepted to critique your solution — not to write it for you.
          </p>
          <div className="mt-3 flex flex-col gap-2 sm:flex-row sm:gap-4">
            <label className="flex items-center gap-2 text-sm">
              <input
                type="radio"
                name="aiProvider"
                checked={aiProvider === "openai"}
                onChange={() => setAiProvider("openai")}
              />
              OpenAI
            </label>
            <label className="flex items-center gap-2 text-sm">
              <input
                type="radio"
                name="aiProvider"
                checked={aiProvider === "ollama"}
                onChange={() => setAiProvider("ollama")}
              />
              Local (Ollama)
            </label>
          </div>

          {aiProvider === "openai" ? (
            <Field label="OpenAI API key">
              <input
                type="password"
                autoComplete="off"
                className="w-full rounded-md border border-ink-700 bg-ink-900 px-3 py-2 font-mono text-sm"
                value={openaiApiKey}
                onChange={(e) => setOpenaiApiKey(e.target.value)}
                placeholder="sk-…"
              />
              {progress?.settings.openaiApiKeySet ? (
                <div className="mt-1 flex flex-wrap items-center gap-x-3 gap-y-1">
                  <p className="text-xs text-paper-500">
                    A key is already saved on this machine. Leave blank to keep it, or enter a new
                    key to replace it.
                  </p>
                  <button
                    type="button"
                    className="text-xs text-paper-400 underline-offset-2 hover:underline"
                    onClick={() => {
                      void (async () => {
                        try {
                          await clearOpenaiKey();
                          setOpenaiApiKey("");
                          setProgress(await loadProgress());
                          setMsg("API key removed.");
                        } catch (e) {
                          setOllamaError(String(e));
                        }
                      })();
                    }}
                  >
                    Remove key
                  </button>
                </div>
              ) : (
                <p className="mt-1 text-xs text-paper-500">Stored only on this machine.</p>
              )}
            </Field>
          ) : (
            <div className="mt-4 space-y-3">
              <Field label="Ollama host">
                <input
                  className="w-full rounded-md border border-ink-700 bg-ink-900 px-3 py-2 font-mono text-sm"
                  value={ollamaHost}
                  onChange={(e) => setOllamaHost(e.target.value)}
                  placeholder={DEFAULT_OLLAMA_HOST}
                />
              </Field>
              <div className="flex flex-wrap items-center gap-2 text-sm">
                <span
                  className={
                    ollamaRunning
                      ? "text-easy"
                      : ollamaChecking
                        ? "text-paper-400"
                        : "text-hard"
                  }
                >
                  {ollamaChecking
                    ? "Checking…"
                    : ollamaRunning
                      ? "Connected"
                      : "Not running"}
                </span>
                <button
                  type="button"
                  className="rounded-md border border-ink-600 px-2 py-1 text-xs text-paper-300 hover:border-gold-700"
                  onClick={() => void refreshOllama(ollamaHost)}
                  disabled={ollamaChecking}
                >
                  Recheck
                </button>
              </div>
              {!ollamaRunning && !ollamaChecking ? (
                <div>
                  <p className="text-xs text-paper-500">
                    Install Ollama on this machine, start it, then recheck. Spar does not install
                    the app for you.
                  </p>
                  <button
                    type="button"
                    className="mt-2 rounded-md bg-gold-400 px-3 py-1.5 text-xs font-semibold text-ink-950"
                    onClick={() => void openExternalUrl("https://ollama.com/download")}
                  >
                    Install Ollama
                  </button>
                </div>
              ) : null}
              {ollamaRunning ? (
                <>
                  <Field label="Model">
                    <select
                      className="w-full rounded-md border border-ink-700 bg-ink-900 px-3 py-2 text-sm"
                      value={ollamaModel}
                      onChange={(e) => {
                        const next = e.target.value;
                        setOllamaModel(next);
                        void saveOllamaSelection({
                          provider: "ollama",
                          host: ollamaHost.trim() || DEFAULT_OLLAMA_HOST,
                          model: next,
                        }).then(() => loadProgress()).then(setProgress);
                      }}
                    >
                      {installedModels.length === 0 ? (
                        <option value="">No models installed</option>
                      ) : (
                        installedModels.map((m) => (
                          <option key={m} value={m}>
                            {m}
                          </option>
                        ))
                      )}
                    </select>
                  </Field>
                  <div className="space-y-2">
                    <p className="text-xs text-paper-400">Recommended models</p>
                    {RECOMMENDED_MODELS.map((m) => {
                      const installed = modelInstalled(m.id);
                      return (
                        <div
                          key={m.id}
                          className="flex items-center justify-between gap-2 rounded-md border border-ink-700 px-3 py-2"
                        >
                          <span className="font-mono text-xs">{m.label}</span>
                          {installed ? (
                            <span className="text-xs text-easy">Installed</span>
                          ) : (
                            <button
                              type="button"
                              className="rounded-md bg-gold-400 px-2 py-1 text-xs font-semibold text-ink-950 disabled:opacity-40"
                              disabled={pulling !== null}
                              onClick={() => void pullModel(m.id)}
                            >
                              {pulling === m.id ? "Installing…" : "Install"}
                            </button>
                          )}
                        </div>
                      );
                    })}
                  </div>
                  {pulling || pullStatus ? (
                    <div className="space-y-2 rounded-md border border-ink-700 bg-ink-950 px-3 py-3">
                      <div className="flex items-start justify-between gap-3">
                        <div className="min-w-0">
                          <p className="truncate font-mono text-xs text-paper-300">
                            {pulling ? `Downloading ${pulling}` : pullStatus}
                          </p>
                          {pullPct != null && pullTotal != null && pullCompleted != null ? (
                            <p className="mt-1 text-xs text-paper-500">
                              {formatBytes(pullCompleted)} / {formatBytes(pullTotal)}
                              {pullRemaining != null
                                ? ` · ${formatBytes(pullRemaining)} remaining`
                                : ""}
                              {` · ${pullPct}%`}
                            </p>
                          ) : pullStatus ? (
                            <p className="mt-1 text-xs text-paper-500">{pullStatus}</p>
                          ) : null}
                        </div>
                        {pulling ? (
                          <button
                            type="button"
                            className="shrink-0 rounded-md border border-ink-600 px-2 py-1 text-xs text-paper-300 hover:border-hard hover:text-hard"
                            onClick={() => void cancelPull()}
                          >
                            Cancel
                          </button>
                        ) : null}
                      </div>
                      <div className="h-2 overflow-hidden rounded-full bg-ink-700">
                        <div
                          className="h-full rounded-full bg-gold-400 transition-[width] duration-300 ease-out"
                          style={{ width: `${pullPct ?? (pulling ? 4 : 0)}%` }}
                        />
                      </div>
                    </div>
                  ) : null}
                </>
              ) : null}
              {ollamaError ? <p className="text-xs text-hard">{ollamaError}</p> : null}
            </div>
          )}
        </div>

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
