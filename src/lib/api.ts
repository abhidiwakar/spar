import { Channel, invoke } from "@tauri-apps/api/core";
import type {
  AttemptComplexity,
  JudgeOutput,
  Language,
  OllamaPullProgress,
  OllamaStatus,
  ProgressSnapshot,
  Runtimes,
  Settings,
} from "./types";

export async function detectRuntimes(pythonPath?: string, nodePath?: string): Promise<Runtimes> {
  return invoke("detect_runtimes", { pythonPath: pythonPath ?? null, nodePath: nodePath ?? null });
}

export async function loadProgress(): Promise<ProgressSnapshot> {
  return invoke("load_progress");
}

export async function saveSettings(settings: Settings): Promise<void> {
  return invoke("save_settings_cmd", { settings });
}

export async function clearOpenaiKey(): Promise<void> {
  return invoke("clear_openai_key_cmd");
}

export async function saveDraft(problemId: string, language: Language, code: string): Promise<void> {
  return invoke("save_draft_cmd", { problemId, language, code });
}

export async function saveNote(problemId: string, body: string): Promise<void> {
  return invoke("save_note_cmd", { problemId, body });
}

export async function submitSolution(input: {
  problemId: string;
  language: Language;
  source: string;
  durationMs?: number;
  pythonPath?: string;
  nodePath?: string;
}): Promise<{ output: JudgeOutput; xp: number }> {
  return invoke("submit_solution", {
    req: {
      problemId: input.problemId,
      language: input.language,
      source: input.source,
      durationMs: input.durationMs ?? null,
      pythonPath: input.pythonPath ?? null,
      nodePath: input.nodePath ?? null,
    },
  });
}

export async function markHintUsed(problemId: string): Promise<void> {
  return invoke("mark_hint_used_cmd", { problemId });
}

export async function loadEditorial(problemId: string): Promise<string> {
  return invoke("load_editorial", { problemId });
}

export async function saveOllamaSelection(input: {
  provider: string;
  host: string;
  model: string;
}): Promise<void> {
  return invoke("save_ollama_selection_cmd", input);
}

export async function reviewSolution(input: {
  problemId: string;
  title: string;
  statement: string;
  tags: string[];
  language: Language;
  code: string;
}): Promise<string> {
  return invoke("review_solution", { req: input });
}

export async function analyzeAttemptComplexity(
  attemptId: string,
  force = false,
): Promise<AttemptComplexity> {
  return invoke("analyze_attempt_complexity", { attemptId, force });
}

export async function ollamaStatus(host: string): Promise<OllamaStatus> {
  return invoke("ollama_status", { host });
}

export async function ollamaPullModel(
  host: string,
  model: string,
  onProgress: (progress: OllamaPullProgress) => void,
): Promise<void> {
  const channel = new Channel<OllamaPullProgress>();
  channel.onmessage = onProgress;
  return invoke("ollama_pull_model", { host, model, onProgress: channel });
}

export async function ollamaCancelPull(): Promise<void> {
  return invoke("ollama_cancel_pull");
}

export async function openExternalUrl(url: string): Promise<void> {
  return invoke("open_external_url", { url });
}

export type { OllamaPullProgress };

export async function runTests(input: {
  problemId: string;
  language: Language;
  source: string;
  pythonPath?: string;
  nodePath?: string;
}): Promise<JudgeOutput> {
  return invoke("run_tests", {
    req: {
      problemId: input.problemId,
      language: input.language,
      source: input.source,
      kind: "run",
      pythonPath: input.pythonPath ?? null,
      nodePath: input.nodePath ?? null,
    },
  });
}
