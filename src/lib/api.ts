import { invoke } from "@tauri-apps/api/core";
import type {
  JudgeOutput,
  Language,
  ProgressSnapshot,
  Runtimes,
  Settings,
  TestCase,
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

export async function saveDraft(problemId: string, language: Language, code: string): Promise<void> {
  return invoke("save_draft_cmd", { problemId, language, code });
}

export async function saveNote(problemId: string, body: string): Promise<void> {
  return invoke("save_note_cmd", { problemId, body });
}

export async function recordAttempt(input: {
  problemId: string;
  language: Language;
  code: string;
  passed: boolean;
  usedHint: boolean;
  durationMs?: number;
  verdict: string;
  difficulty: string;
}): Promise<number> {
  return invoke("record_attempt_cmd", { attempt: input });
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

export async function runTests(input: {
  language: Language;
  source: string;
  entry: string;
  mode: string;
  helpers: string[];
  paramNames: string[];
  tests: TestCase[];
  pythonPath?: string;
  nodePath?: string;
}): Promise<JudgeOutput> {
  return invoke("run_tests", {
    req: {
      language: input.language,
      source: input.source,
      entry: input.entry,
      mode: input.mode,
      helpers: input.helpers,
      paramNames: input.paramNames,
      tests: input.tests,
      pythonPath: input.pythonPath ?? null,
      nodePath: input.nodePath ?? null,
    },
  });
}
