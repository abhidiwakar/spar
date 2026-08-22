export type Difficulty = "easy" | "medium" | "hard";
export type Language = "python" | "javascript";
export type ProblemKind = "core" | "stretch";
export type UnitKind = "standard" | "mixed" | "mock";

export type TestCase = {
  args?: Record<string, unknown>;
  expected: unknown;
  compare?: "exact" | "sorted" | "set";
  extra?: Record<string, unknown>;
  ops?: unknown[];
};

export type Problem = {
  id: string;
  title: string;
  difficulty: Difficulty;
  tags: string[];
  unitId: string;
  kind: ProblemKind;
  order: number;
  microsoftNote: string;
  statement: string;
  examples: { input: string; output: string; explanation?: string }[];
  constraints: string[];
  followUp?: string;
  editorial?: string;
  mode: "function" | "class";
  entry: { python: string; javascript: string };
  paramNames: string[];
  helpers: string[];
  starter: { python: string; javascript: string };
  tests: { visible: TestCase[]; hidden: TestCase[] };
};

export type Unit = {
  id: string;
  week: number;
  title: string;
  subtitle: string;
  briefing: string;
  kind: UnitKind;
  coreIds: string[];
  stretchIds: string[];
  timerMinutes?: number;
};

export type ProblemState = {
  problemId: string;
  status: string;
  firstAcceptedAt?: string | null;
  lastAttemptAt?: string | null;
  usedHint: boolean;
  reviewAt?: string | null;
};

export type Attempt = {
  id: string;
  problemId: string;
  language: string;
  code: string;
  passed: boolean;
  usedHint: boolean;
  durationMs?: number | null;
  createdAt: string;
  verdict: string;
};

export type AiProvider = "openai" | "ollama";

export type Settings = {
  pythonPath: string;
  nodePath: string;
  defaultLanguage: Language | string;
  dailyGoal: number;
  timerEnabled: boolean;
  openaiApiKey?: string;
  openaiApiKeySet: boolean;
  aiProvider: AiProvider | string;
  ollamaHost: string;
  ollamaModel: string;
};

export type ProgressSnapshot = {
  settings: Settings;
  problemStates: ProblemState[];
  attempts: Attempt[];
  daily: { date: string; xp: number; goalMet: boolean; acceptedCount: number }[];
  drafts: { problemId: string; language: string; code: string }[];
  notes: { problemId: string; body: string }[];
  aiReviews: { problemId: string; language: string; body: string }[];
  attemptComplexities: {
    attemptId: string;
    timeComplexity: string;
    spaceComplexity: string;
  }[];
  streak: number;
  xpToday: number;
  today: string;
};

export type AttemptComplexity = {
  attemptId: string;
  timeComplexity: string;
  spaceComplexity: string;
};

export type OllamaStatus = {
  running: boolean;
  models: string[];
};

export type OllamaPullProgress = {
  status: string;
  completed?: number | null;
  total?: number | null;
};

export type JudgeOutput = {
  verdict: string;
  passed: number;
  total: number;
  stdout: string;
  stderr: string;
  cases: {
    index: number;
    passed: boolean;
    expected: unknown;
    actual: unknown;
    error?: string | null;
    stdout?: string | null;
  }[];
};

export type Runtimes = {
  python: string | null;
  node: string | null;
};
