import { type ReactNode, useEffect } from "react";
import { detectRuntimes, loadProgress } from "./lib/api";
import { units } from "./lib/content";
import { useApp } from "./lib/store";
import type { Language } from "./lib/types";
import sparMark from "./assets/spar-mark.svg";
import { HomeScreen } from "./features/home/HomeScreen";
import { PathScreen } from "./features/path/PathScreen";
import { WorkspaceScreen } from "./features/workspace/WorkspaceScreen";
import { ProgressScreen } from "./features/progress/ProgressScreen";
import { SettingsScreen } from "./features/settings/SettingsScreen";
import { SetupScreen } from "./features/setup/SetupScreen";

export default function App() {
  const screen = useApp((s) => s.screen);
  const setScreen = useApp((s) => s.setScreen);
  const setRuntimes = useApp((s) => s.setRuntimes);
  const setProgress = useApp((s) => s.setProgress);
  const setLanguage = useApp((s) => s.setLanguage);
  const progress = useApp((s) => s.progress);
  const runtimes = useApp((s) => s.runtimes);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const snap = await loadProgress();
        if (cancelled) return;
        setProgress(snap);
        const rt = await detectRuntimes(snap.settings.pythonPath, snap.settings.nodePath);
        if (cancelled) return;
        setRuntimes(rt);
        const preferred = snap.settings.defaultLanguage as Language;
        if (preferred === "javascript" && rt.node) setLanguage("javascript");
        else if (preferred === "python" && rt.python) setLanguage("python");
        else if (rt.python) setLanguage("python");
        else if (rt.node) setLanguage("javascript");
        if (!rt.python && !rt.node) setScreen("setup");
      } catch {
        const rt = await detectRuntimes();
        if (cancelled) return;
        setRuntimes(rt);
        if (!rt.python && !rt.node) setScreen("setup");
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [setLanguage, setProgress, setRuntimes, setScreen]);

  if (screen === "setup" || (runtimes && !runtimes.python && !runtimes.node && screen !== "settings")) {
    return <SetupScreen />;
  }

  return (
    <div className="flex h-full min-h-0 flex-col bg-ink-950 text-paper-100">
      <header className="flex h-12 shrink-0 items-center justify-between border-b border-ink-700 px-4">
        <button
          className="flex items-center gap-2 text-sm font-semibold tracking-tight"
          onClick={() => setScreen("home")}
        >
          <img src={sparMark} alt="" className="h-6 w-6 rounded-[5px]" />
          Spar
        </button>
        <nav className="flex items-center gap-1 text-sm text-paper-400">
          <NavBtn active={screen === "home"} onClick={() => setScreen("home")}>
            Home
          </NavBtn>
          <NavBtn active={screen === "path"} onClick={() => setScreen("path")}>
            Path
          </NavBtn>
          <NavBtn active={screen === "progress"} onClick={() => setScreen("progress")}>
            Progress
          </NavBtn>
          <NavBtn active={screen === "settings"} onClick={() => setScreen("settings")}>
            Settings
          </NavBtn>
        </nav>
        <div className="flex items-center gap-4 text-xs text-paper-400">
          <span>
            Streak <strong className="text-gold-400">{progress?.streak ?? 0}</strong>
          </span>
          <span>
            Today <strong className="text-paper-100">{progress?.xpToday ?? 0} XP</strong>
          </span>
          <span className="hidden sm:inline text-paper-500">
            {units.length} units
          </span>
        </div>
      </header>
      <main className="min-h-0 flex-1">
        {screen === "home" && <HomeScreen />}
        {screen === "path" && <PathScreen />}
        {screen === "workspace" && <WorkspaceScreen />}
        {screen === "progress" && <ProgressScreen />}
        {screen === "settings" && <SettingsScreen />}
      </main>
    </div>
  );
}

function NavBtn({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      className={`rounded-md px-3 py-1.5 ${
        active ? "bg-ink-800 text-paper-100" : "hover:bg-ink-850 hover:text-paper-100"
      }`}
    >
      {children}
    </button>
  );
}
