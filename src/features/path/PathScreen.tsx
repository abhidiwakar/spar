import { problemsForUnit, units } from "../../lib/content";
import { useApp } from "../../lib/store";
import { unitUnlocked } from "../../lib/path";

export function PathScreen() {
  const progress = useApp((s) => s.progress);
  const openProblem = useApp((s) => s.openProblem);
  const accepted = new Set(
    (progress?.problemStates ?? []).filter((s) => s.status === "accepted").map((s) => s.problemId),
  );
  const attempted = new Set(
    (progress?.problemStates ?? []).filter((s) => s.status === "attempted").map((s) => s.problemId),
  );

  return (
    <div className="h-full overflow-auto">
      <div className="mx-auto max-w-2xl px-6 py-10">
        <h1 className="font-serif text-3xl">The path</h1>
        <p className="mt-2 text-sm text-paper-400">
          Twelve units. Finish a unit's core problems to unlock the next. Stretch never blocks you.
        </p>
        <ol className="mt-10 space-y-8">
          {units.map((unit) => {
            const unlocked = unitUnlocked(unit, accepted);
            const list = problemsForUnit(unit);
            const coreDone = unit.coreIds.filter((id) => accepted.has(id)).length;
            return (
              <li key={unit.id} className="relative pl-8">
                <span
                  className={`absolute left-0 top-1 h-3 w-3 rounded-full ${
                    coreDone === unit.coreIds.length && unit.coreIds.length
                      ? "bg-gold-400"
                      : unlocked
                        ? "border-2 border-gold-400 bg-ink-950"
                        : "border-2 border-ink-700 bg-ink-950"
                  }`}
                />
                <div className={unlocked ? "" : "opacity-50"}>
                  <p className="text-xs uppercase tracking-wider text-paper-500">
                    Week {unit.week}
                    {unit.kind !== "standard" ? ` · ${unit.kind}` : ""}
                  </p>
                  <h2 className="mt-1 text-lg font-medium">{unit.title}</h2>
                  <p className="mt-1 text-sm text-paper-400">{unit.briefing}</p>
                  <p className="mt-2 text-xs text-paper-500">
                    {coreDone}/{unit.coreIds.length} core
                  </p>
                  <ul className="mt-3 space-y-1">
                    {list.map((p) => {
                      const done = accepted.has(p.id);
                      const tried = attempted.has(p.id);
                      return (
                        <li key={p.id}>
                          <button
                            disabled={!unlocked}
                            onClick={() => openProblem(p.id)}
                            className="flex w-full items-center justify-between rounded-md px-2 py-1.5 text-left text-sm hover:bg-ink-850 disabled:cursor-not-allowed"
                          >
                            <span>
                              <span className="mr-2 text-paper-500">
                                {done ? "AC" : tried ? "—" : "○"}
                              </span>
                              {p.title}
                              {p.kind === "stretch" ? (
                                <span className="ml-2 text-xs text-paper-500">stretch</span>
                              ) : null}
                            </span>
                            <Diff d={p.difficulty} />
                          </button>
                        </li>
                      );
                    })}
                  </ul>
                </div>
              </li>
            );
          })}
        </ol>
      </div>
    </div>
  );
}

function Diff({ d }: { d: string }) {
  const color = d === "easy" ? "text-easy" : d === "hard" ? "text-hard" : "text-medium";
  return <span className={`text-xs capitalize ${color}`}>{d}</span>;
}
