import { create } from "zustand";
import type { CatalogSnapshot } from "./content";
import type { Language, ProgressSnapshot, Runtimes } from "./types";

export type Screen = "setup" | "home" | "path" | "workspace" | "progress" | "settings";

type AppState = {
  screen: Screen;
  problemId: string | null;
  language: Language;
  runtimes: Runtimes | null;
  progress: ProgressSnapshot | null;
  catalog: CatalogSnapshot | null;
  usedHint: boolean;
  setScreen: (s: Screen) => void;
  openProblem: (id: string) => void;
  setLanguage: (l: Language) => void;
  setRuntimes: (r: Runtimes) => void;
  setProgress: (p: ProgressSnapshot) => void;
  setCatalog: (c: CatalogSnapshot) => void;
  setUsedHint: (v: boolean) => void;
};

export const useApp = create<AppState>((set) => ({
  screen: "home",
  problemId: null,
  language: "python",
  runtimes: null,
  progress: null,
  catalog: null,
  usedHint: false,
  setScreen: (screen) => set({ screen }),
  openProblem: (problemId) => set({ screen: "workspace", problemId }),
  setLanguage: (language) => set({ language }),
  setRuntimes: (runtimes) => set({ runtimes }),
  setProgress: (progress) => set({ progress }),
  setCatalog: (catalog) => set({ catalog }),
  setUsedHint: (usedHint) => set({ usedHint }),
}));
