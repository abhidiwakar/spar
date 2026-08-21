import Editor from "@monaco-editor/react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import Markdown from "react-markdown";
import { Panel, PanelGroup, PanelResizeHandle } from "react-resizable-panels";
import remarkGfm from "remark-gfm";
import { loadProgress, recordAttempt, reviewSolution, runTests, saveDraft, saveNote } from "../../lib/api";
import { problemById, unitById } from "../../lib/content";
import { useApp } from "../../lib/store";
import type { JudgeOutput, Language, TestCase } from "../../lib/types";

type LeftTab = "description" | "editorial" | "submissions" | "review";

export function WorkspaceScreen() {
  const problemId = useApp((s) => s.problemId);
  const language = useApp((s) => s.language);
  const setLanguage = useApp((s) => s.setLanguage);
  const progress = useApp((s) => s.progress);
  const setProgress = useApp((s) => s.setProgress);
  const setScreen = useApp((s) => s.setScreen);
  const runtimes = useApp((s) => s.runtimes);
  const usedHint = useApp((s) => s.usedHint);
  const setUsedHint = useApp((s) => s.setUsedHint);
  const problem = problemId ? problemById.get(problemId) : undefined;

  const [code, setCode] = useState("");
  const [leftTab, setLeftTab] = useState<LeftTab>("description");
  const [activeCase, setActiveCase] = useState(0);
  const [customCases, setCustomCases] = useState<TestCase[]>([]);
  const [result, setResult] = useState<JudgeOutput | null>(null);
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState("");
  const [reviewText, setReviewText] = useState("");
  const [reviewError, setReviewError] = useState("");
  const [reviewing, setReviewing] = useState(false);
  const [elapsed, setElapsed] = useState(0);
  const [timerOn, setTimerOn] = useState(false);
  const started = useRef(Date.now());
  const saveTimer = useRef<number | null>(null);

  const accepted = progress?.problemStates.some(
    (s) => s.problemId === problemId && s.status === "accepted",
  );
  const unit = problem ? unitById(problem.unitId) : undefined;
  const timerMinutes = unit?.timerMinutes ?? 25;
  const missingRuntime =
    language === "python" ? !runtimes?.python : language === "javascript" ? !runtimes?.node : true;

  useEffect(() => {
    if (!problem) return;
    const d = progress?.drafts.find((x) => x.problemId === problem.id && x.language === language);
    setCode(d?.code || problem.starter[language]);
    setResult(null);
    setActiveCase(0);
    setCustomCases(problem.tests.visible.map((t) => ({ ...t })));
    setLeftTab("description");
    setReviewText("");
    setReviewError("");
    started.current = Date.now();
    setElapsed(0);
    const n = progress?.notes.find((x) => x.problemId === problem.id);
    setNote(n?.body ?? "");
  }, [problem, language]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    if (!progress?.settings.timerEnabled) {
      setTimerOn(false);
      return;
    }
    setTimerOn(true);
  }, [problemId, progress?.settings.timerEnabled]);

  useEffect(() => {
    if (!timerOn) return;
    const id = window.setInterval(() => setElapsed(Math.floor((Date.now() - started.current) / 1000)), 1000);
    return () => window.clearInterval(id);
  }, [timerOn, problemId]);

  const persistDraft = useCallback(
    (value: string) => {
      if (!problem) return;
      if (saveTimer.current) window.clearTimeout(saveTimer.current);
      saveTimer.current = window.setTimeout(() => {
        void saveDraft(problem.id, language, value);
      }, 400);
    },
    [language, problem],
  );

  const cases = customCases.length ? customCases : problem?.tests.visible ?? [];

  async function run(kind: "run" | "submit") {
    if (!problem || missingRuntime) return;
    setBusy(true);
    setResult(null);
    try {
      const tests =
        kind === "run" ? cases : [...problem.tests.visible, ...problem.tests.hidden];
      const out = await runTests({
        language,
        source: code,
        entry: problem.entry[language],
        mode: problem.mode,
        helpers: problem.helpers,
        paramNames: problem.paramNames,
        tests,
        pythonPath: progress?.settings.pythonPath,
        nodePath: progress?.settings.nodePath,
      });
      setResult(out);
      if (kind === "submit") {
        const awarded = await recordAttempt({
          problemId: problem.id,
          language,
          code,
          passed: out.verdict === "accepted",
          usedHint,
          durationMs: Date.now() - started.current,
          verdict: out.verdict,
          difficulty: problem.difficulty,
        });
        void awarded;
        await saveDraft(problem.id, language, code);
        setProgress(await loadProgress());
      }
    } catch (e) {
      const message = String(e);
      const friendly = /missing field/i.test(message)
        ? "Your code did not return a value the judge could read. Check that the function exists and returns an answer."
        : message;
      setResult({
        verdict: "runtime_error",
        passed: 0,
        total: 0,
        stdout: "",
        stderr: friendly,
        cases: [],
      });
    } finally {
      setBusy(false);
    }
  }

  async function requestReview() {
    if (!problem) return;
    setReviewing(true);
    setReviewError("");
    try {
      const text = await reviewSolution({
        problemId: problem.id,
        title: problem.title,
        statement: problem.statement,
        tags: problem.tags,
        language,
        code,
      });
      setReviewText(text);
    } catch (e) {
      setReviewError(String(e));
    } finally {
      setReviewing(false);
    }
  }

  const submissions = useMemo(
    () => (progress?.attempts ?? []).filter((a) => a.problemId === problemId),
    [progress, problemId],
  );

  if (!problem) {
    return (
      <div className="flex h-full items-center justify-center text-paper-400">
        Pick a problem from the path.
      </div>
    );
  }

  const diffColor =
    problem.difficulty === "easy"
      ? "text-easy"
      : problem.difficulty === "hard"
        ? "text-hard"
        : "text-medium";

  const mm = String(Math.floor(elapsed / 60)).padStart(2, "0");
  const ss = String(elapsed % 60).padStart(2, "0");
  const over = elapsed > timerMinutes * 60;

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex h-11 shrink-0 items-center justify-between border-b border-ink-700 px-3">
        <div className="flex min-w-0 items-center gap-3">
          <h1 className="truncate text-sm font-medium">{problem.title}</h1>
          <span className={`text-xs capitalize ${diffColor}`}>{problem.difficulty}</span>
          {accepted ? <span className="text-xs text-easy">Accepted</span> : null}
        </div>
        <div className="flex items-center gap-2">
          {progress?.settings.timerEnabled ? (
            <span className={`font-mono text-xs ${over ? "text-hard" : "text-paper-400"}`}>
              {mm}:{ss}
            </span>
          ) : null}
          <select
            className="rounded-md border border-ink-700 bg-ink-900 px-2 py-1 text-xs"
            value={language}
            onChange={(e) => setLanguage(e.target.value as Language)}
          >
            <option value="python">Python</option>
            <option value="javascript">JavaScript</option>
          </select>
          <button
            className="rounded-md border border-ink-700 px-3 py-1 text-xs hover:bg-ink-800 disabled:opacity-40"
            disabled={busy || missingRuntime}
            onClick={() => void run("run")}
          >
            Run
          </button>
          <button
            className="rounded-md bg-gold-400 px-3 py-1 text-xs font-semibold text-ink-950 hover:bg-gold-500 disabled:opacity-40"
            disabled={busy || missingRuntime}
            onClick={() => void run("submit")}
          >
            Submit
          </button>
        </div>
      </div>

      {missingRuntime ? (
        <div className="border-b border-hard/40 bg-hard/10 px-3 py-2 text-xs text-hard">
          {language === "python" ? "Python" : "Node.js"} was not found. Set a path in Settings to Run
          and Submit.
        </div>
      ) : null}

      <PanelGroup direction="horizontal" className="min-h-0 flex-1">
        <Panel defaultSize={42} minSize={26}>
          <div className="flex h-full min-h-0 flex-col bg-ink-900">
            <div className="flex shrink-0 gap-1 border-b border-ink-700 px-2 pt-2">
              {(["description", "editorial", "submissions", "review"] as LeftTab[]).map((t) => (
                <button
                  key={t}
                  className={`rounded-t-md px-3 py-1.5 text-xs capitalize ${
                    leftTab === t ? "bg-ink-850 text-paper-100" : "text-paper-400"
                  }`}
                  onClick={() => setLeftTab(t)}
                >
                  {t}
                </button>
              ))}
            </div>
            <div className="min-h-0 flex-1 overflow-auto px-5 py-4">
              {leftTab === "description" && (
                <div>
                  <div className="flex flex-wrap gap-1.5">
                    {problem.tags.map((t) => (
                      <span key={t} className="rounded bg-ink-800 px-2 py-0.5 text-[11px] text-paper-400">
                        {t}
                      </span>
                    ))}
                  </div>
                  <p className="mt-3 text-xs text-paper-500">{problem.microsoftNote}</p>
                  <div className="problem-prose mt-4">
                    <Markdown remarkPlugins={[remarkGfm]}>{problem.statement}</Markdown>
                  </div>
                  <h3 className="mt-6 text-xs font-semibold uppercase tracking-wider text-paper-400">
                    Examples
                  </h3>
                  {problem.examples.map((ex, i) => (
                    <div key={i} className="mt-3 rounded-lg border border-ink-700 bg-ink-950 p-3 text-sm">
                      <div className="font-mono text-xs text-paper-400">Example {i + 1}</div>
                      <div className="mt-2 font-mono text-[13px]">
                        <div>
                          <span className="text-paper-500">Input:</span> {ex.input}
                        </div>
                        <div>
                          <span className="text-paper-500">Output:</span> {ex.output}
                        </div>
                        {ex.explanation ? (
                          <div className="mt-1 text-paper-400">{ex.explanation}</div>
                        ) : null}
                      </div>
                    </div>
                  ))}
                  <h3 className="mt-6 text-xs font-semibold uppercase tracking-wider text-paper-400">
                    Constraints
                  </h3>
                  <ul className="mt-2 list-disc pl-5 font-mono text-xs text-paper-400">
                    {problem.constraints.map((c) => (
                      <li key={c}>{c}</li>
                    ))}
                  </ul>
                  {problem.followUp ? (
                    <p className="mt-4 text-sm text-paper-400">
                      <strong className="text-paper-100">Follow-up:</strong> {problem.followUp}
                    </p>
                  ) : null}
                  <button
                    className="mt-6 text-xs text-gold-400 underline-offset-2 hover:underline"
                    onClick={() => setUsedHint(true)}
                  >
                    {usedHint ? "Hint used (no XP bonus)" : "Reveal approach hint"}
                  </button>
                  {usedHint && problem.editorial ? (
                    <p className="mt-2 text-sm text-paper-400">{problem.editorial.split("\n")[0]}</p>
                  ) : null}
                </div>
              )}
              {leftTab === "editorial" && (
                <div className="problem-prose">
                  {accepted || usedHint ? (
                    <Markdown remarkPlugins={[remarkGfm]}>
                      {problem.editorial || "No editorial for this problem yet."}
                    </Markdown>
                  ) : (
                    <p className="text-sm text-paper-400">
                      Editorial unlocks after you submit, or if you opt into a hint.
                    </p>
                  )}
                  <div className="mt-6">
                    <p className="text-xs text-paper-400">Pattern note (saved locally)</p>
                    <textarea
                      className="mt-2 h-28 w-full rounded-md border border-ink-700 bg-ink-950 p-2 font-sans text-sm"
                      value={note}
                      onChange={(e) => setNote(e.target.value)}
                      onBlur={() => void saveNote(problem.id, note)}
                    />
                  </div>
                </div>
              )}
              {leftTab === "submissions" && (
                <ul className="space-y-2 text-sm">
                  {submissions.length === 0 ? (
                    <li className="text-paper-400">No submissions yet.</li>
                  ) : (
                    submissions.map((a) => (
                      <li key={a.id} className="rounded-md border border-ink-700 px-3 py-2">
                        <div className="flex justify-between">
                          <span className={a.passed ? "text-easy" : "text-hard"}>{a.verdict}</span>
                          <span className="text-xs text-paper-500">
                            {a.language} · {new Date(a.createdAt).toLocaleString()}
                          </span>
                        </div>
                      </li>
                    ))
                  )}
                </ul>
              )}
              {leftTab === "review" && (
                <div>
                  {!accepted ? (
                    <p className="text-sm text-paper-400">
                      Review unlocks after you get Accepted on this problem.
                    </p>
                  ) : !(progress?.settings.openaiApiKey ?? "").trim() ? (
                    <div>
                      <p className="text-sm text-paper-400">
                        Add an OpenAI API key in Settings to review your solution.
                      </p>
                      <button
                        className="mt-4 rounded-md bg-gold-400 px-3 py-1.5 text-xs font-semibold text-ink-950"
                        onClick={() => setScreen("settings")}
                      >
                        Open Settings
                      </button>
                    </div>
                  ) : (
                    <div>
                      <p className="text-sm text-paper-400">
                        Critique of the code in the editor: complexity, edge cases, and a cleaner
                        approach. It will not write a full replacement solution.
                      </p>
                      <button
                        className="mt-4 rounded-md bg-gold-400 px-3 py-1.5 text-xs font-semibold text-ink-950 disabled:opacity-40"
                        disabled={reviewing}
                        onClick={() => void requestReview()}
                      >
                        {reviewing
                          ? "Reviewing…"
                          : reviewText
                            ? "Review again"
                            : "Review this code"}
                      </button>
                      {reviewError ? (
                        <p className="mt-3 whitespace-pre-wrap text-sm text-hard">{reviewError}</p>
                      ) : null}
                      {reviewText ? (
                        <div className="problem-prose mt-5">
                          <Markdown remarkPlugins={[remarkGfm]}>{reviewText}</Markdown>
                        </div>
                      ) : null}
                    </div>
                  )}
                </div>
              )}
            </div>
          </div>
        </Panel>
        <PanelResizeHandle className="w-1 bg-ink-700 hover:bg-gold-700" />
        <Panel minSize={30}>
          <PanelGroup direction="vertical">
            <Panel defaultSize={68} minSize={30}>
              <Editor
                theme="vs-dark"
                language={language === "python" ? "python" : "javascript"}
                value={code}
                onChange={(v) => {
                  const next = v ?? "";
                  setCode(next);
                  persistDraft(next);
                }}
                options={{
                  minimap: { enabled: false },
                  fontSize: 13,
                  fontFamily: "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
                  scrollBeyondLastLine: false,
                  automaticLayout: true,
                  tabSize: 4,
                  padding: { top: 12 },
                }}
              />
            </Panel>
            <PanelResizeHandle className="h-1 bg-ink-700 hover:bg-gold-700" />
            <Panel defaultSize={32} minSize={18}>
              <div className="flex h-full min-h-0 flex-col bg-ink-900">
                <div className="flex shrink-0 items-center justify-between border-b border-ink-700 px-3 py-1.5">
                  <div className="flex gap-1 overflow-auto">
                    {cases.map((_, i) => (
                      <button
                        key={i}
                        className={`rounded px-2 py-1 text-[11px] ${
                          activeCase === i ? "bg-ink-800 text-paper-100" : "text-paper-400"
                        }`}
                        onClick={() => setActiveCase(i)}
                      >
                        Case {i + 1}
                        {result?.cases[i]
                          ? result.cases[i].passed
                            ? " ✓"
                            : " ✕"
                          : ""}
                      </button>
                    ))}
                  </div>
                  {result ? (
                    <span
                      className={`text-xs ${
                        result.verdict === "accepted" ? "text-easy" : "text-hard"
                      }`}
                    >
                      {labelVerdict(result.verdict)} · {result.passed}/{result.total}
                    </span>
                  ) : (
                    <span className="text-xs text-paper-500">{busy ? "Running…" : "Testcases"}</span>
                  )}
                </div>
                <div className="min-h-0 flex-1 overflow-auto p-3 font-mono text-xs">
                  {cases[activeCase] ? (
                    <div>
                      <p className="text-paper-500">Input</p>
                      <pre className="mt-1 whitespace-pre-wrap text-paper-100">
                        {formatCase(cases[activeCase])}
                      </pre>
                      <p className="mt-3 text-paper-500">Expected</p>
                      <pre className="mt-1">{JSON.stringify(cases[activeCase].expected)}</pre>
                      {result?.cases[activeCase] ? (
                        <>
                          <p className="mt-3 text-paper-500">Output</p>
                          <pre className="mt-1">
                            {formatActual(
                              result.cases[activeCase].actual,
                              result.cases[activeCase].passed,
                            )}
                          </pre>
                          {result.cases[activeCase].error ? (
                            <pre className="mt-2 whitespace-pre-wrap text-hard">
                              {result.cases[activeCase].error}
                            </pre>
                          ) : null}
                          <p className="mt-3 text-paper-500">Stdout</p>
                          <pre className="mt-1 whitespace-pre-wrap text-paper-300">
                            {formatStdout(result.cases[activeCase].stdout, result.stdout)}
                          </pre>
                        </>
                      ) : result?.stdout ? (
                        <>
                          <p className="mt-3 text-paper-500">Stdout</p>
                          <pre className="mt-1 whitespace-pre-wrap text-paper-300">{result.stdout}</pre>
                        </>
                      ) : null}
                      {result?.stderr ? (
                        <pre className="mt-3 whitespace-pre-wrap text-hard">{result.stderr}</pre>
                      ) : null}
                    </div>
                  ) : (
                    <p className="text-paper-500">No testcases.</p>
                  )}
                </div>
              </div>
            </Panel>
          </PanelGroup>
        </Panel>
      </PanelGroup>
    </div>
  );
}

function formatCase(t: TestCase): string {
  if (t.ops) return JSON.stringify(t.ops, null, 2);
  if (t.args) return JSON.stringify(t.args, null, 2);
  return "";
}

function formatActual(actual: unknown, passed: boolean): string {
  if ((actual === null || actual === undefined) && !passed) {
    return "undefined (no return)";
  }
  return JSON.stringify(actual);
}

function formatStdout(caseOut?: string | null, runOut?: string): string {
  const fromCase = caseOut?.trim();
  if (fromCase) return caseOut ?? fromCase;
  const fromRun = runOut?.trim();
  if (fromRun) return runOut ?? fromRun;
  return "(empty)";
}

function labelVerdict(v: string): string {
  if (v === "accepted") return "Accepted";
  if (v === "wrong_answer") return "Wrong Answer";
  if (v === "tle") return "Time Limit Exceeded";
  return "Runtime Error";
}
