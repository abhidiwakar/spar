import { problemById, problems, problemsForUnit, units } from "../../lib/content";
import { useApp } from "../../lib/store";
import type { Problem, Unit } from "../../lib/types";

export function HomeScreen() {
  const progress = useApp((s) => s.progress);
  const openProblem = useApp((s) => s.openProblem);
  const setScreen = useApp((s) => s.setScreen);
  const next = nextProblem(progress?.problemStates ?? []);
  const accepted = new Set(
    (progress?.problemStates ?? []).filter((s) => s.status === "accepted").map((s) => s.problemId),
  );
  const goal = progress?.settings.dailyGoal ?? 1;
  const today = progress?.daily.find((d) => d.date === isoToday());
  const goalMet = Boolean(today?.goalMet);

  return (
    <div className="mx-auto flex h-full max-w-3xl flex-col justify-center gap-10 px-6 py-10">
      <div>
        <p className="text-xs uppercase tracking-[0.2em] text-gold-400">Microsoft interview gym</p>
        <h1 className="mt-2 font-serif text-4xl leading-tight">Solve the next one.</h1>
        <p className="mt-3 max-w-xl text-paper-400">
          A 12-week path of medium-first problems. Trees and graphs are weighted the way Microsoft
          actually asks them. Write Python or JavaScript, run tests, submit.
        </p>
      </div>

      <div className="grid grid-cols-3 gap-3">
        <Stat label="Streak" value={`${progress?.streak ?? 0}d`} />
        <Stat label="XP today" value={`${progress?.xpToday ?? 0}`} />
        <Stat label="Accepted" value={`${accepted.size}/${problems.length}`} />
      </div>

      <div className="rounded-xl border border-ink-700 bg-ink-900 p-5">
        <div className="flex items-center justify-between gap-4">
          <div>
            <p className="text-xs text-paper-400">{goalMet ? "Daily goal complete" : "Today's problem"}</p>
            <p className="mt-1 text-lg font-medium">{next ? next.title : "Path complete"}</p>
            {next && (
              <p className="mt-1 text-sm text-paper-400">
                {next.difficulty} · {next.tags.join(" · ")}
              </p>
            )}
          </div>
          <button
            className="rounded-md bg-gold-400 px-5 py-2.5 text-sm font-semibold text-ink-950 hover:bg-gold-500"
            onClick={() => (next ? openProblem(next.id) : setScreen("path"))}
          >
            Continue
          </button>
        </div>
        <p className="mt-4 text-xs text-paper-500">
          Daily goal: {today?.acceptedCount ?? 0}/{goal} accepted · no hearts, unlimited submits
        </p>
      </div>
    </div>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-xl border border-ink-700 bg-ink-900 px-4 py-3">
      <div className="text-xs text-paper-400">{label}</div>
      <div className="mt-1 text-xl font-semibold">{value}</div>
    </div>
  );
}

function isoToday() {
  const d = new Date();
  return d.toISOString().slice(0, 10);
}

function nextProblem(
  states: { problemId: string; status: string }[],
): Problem | undefined {
  const accepted = new Set(states.filter((s) => s.status === "accepted").map((s) => s.problemId));
  for (const unit of units) {
    if (!unitUnlocked(unit, accepted)) continue;
    const list = problemsForUnit(unit);
    const undone = list.find((p) => p.kind === "core" && !accepted.has(p.id));
    if (undone) return undone;
  }
  for (const unit of units) {
    const stretch = problemsForUnit(unit).find((p) => p.kind === "stretch" && !accepted.has(p.id));
    if (stretch) return stretch;
  }
  return problemById.get("two-sum") ?? problems[0];
}

export function unitUnlocked(unit: Unit, accepted: Set<string>): boolean {
  if (unit.week <= 1) return true;
  const prev = units.find((u) => u.week === unit.week - 1);
  if (!prev) return true;
  return prev.coreIds.every((id) => accepted.has(id));
}
