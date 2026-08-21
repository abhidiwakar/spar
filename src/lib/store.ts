import { create } from "zustand";
import type { Language, ProgressSnapshot, Runtimes } from "./types";

export type Screen = "setup" | "home" | "path" | "workspace" | "progress" | "settings";

type AppState = {
  screen: Screen;
  problemId: string | null;
  language: Language;
  runtimes: Runtimes | null;
  progress: ProgressSnapshot | null;
  usedHint: boolean;
  setScreen: (s: Screen) => void;
  openProblem: (id: string) => void;
  setLanguage: (l: Language) => void;
  setRuntimes: (r: Runtimes) => void;
  setProgress: (p: ProgressSnapshot) => void;
  setUsedHint: (v: boolean) => void;
};

export const useApp = create<AppState>((set) => ({
  screen: "home",
  problemId: null,
  language: "python",
  runtimes: null,
  progress: null,
  usedHint: false,
  setScreen: (screen) => set({ screen }),
  openProblem: (problemId) => set({ screen: "workspace", problemId, usedHint: false }),
  setLanguage: (language) => set({ language }),
  setRuntimes: (runtimes) => set({ runtimes }),
  setProgress: (progress) => set({ progress }),
  setUsedHint: (usedHint) => set({ usedHint }),
}));
