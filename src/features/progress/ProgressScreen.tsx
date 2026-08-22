import { problems, units } from "../../lib/content";
import { lastNLocalDates } from "../../lib/dates";
import { useApp } from "../../lib/store";

export function ProgressScreen() {
  const progress = useApp((s) => s.progress);
  const openProblem = useApp((s) => s.openProblem);
  const accepted = new Set(
    (progress?.problemStates ?? []).filter((s) => s.status === "accepted").map((s) => s.problemId),
  );
  const byTag = new Map<string, { total: number; ac: number }>();
  for (const p of problems) {
    for (const t of p.tags) {
      const cur = byTag.get(t) ?? { total: 0, ac: 0 };
      cur.total += 1;
      if (accepted.has(p.id)) cur.ac += 1;
      byTag.set(t, cur);
    }
  }
  const days = lastNLocalDates(42, progress?.today);
  const daily = new Map((progress?.daily ?? []).map((d) => [d.date, d]));

  return (
    <div className="h-full overflow-auto">
      <div className="mx-auto max-w-3xl px-6 py-10">
        <h1 className="font-serif text-3xl">Progress</h1>
        <p className="mt-2 text-sm text-paper-400">
          {accepted.size} accepted · streak {progress?.streak ?? 0} · {units.length} units
        </p>
        <h2 className="mt-8 text-sm font-medium text-paper-400">Last 6 weeks</h2>
        <div className="mt-3 flex flex-wrap gap-1">
          {days.map((d) => {
            const hit = daily.get(d);
            const bg = !hit ? "bg-ink-800" : hit.goalMet ? "bg-gold-400" : "bg-gold-700";
            return <span key={d} title={`${d} · ${hit?.xp ?? 0} XP`} className={`h-3 w-3 rounded-sm ${bg}`} />;
          })}
        </div>
        <h2 className="mt-8 text-sm font-medium text-paper-400">By pattern</h2>
        <ul className="mt-3 space-y-2">
          {[...byTag.entries()]
            .sort((a, b) => b[1].total - a[1].total)
            .map(([tag, v]) => (
              <li key={tag} className="flex items-center gap-3 text-sm">
                <span className="w-40 text-paper-400">{tag}</span>
                <span className="h-1.5 flex-1 overflow-hidden rounded bg-ink-800">
                  <span
                    className="block h-full bg-gold-400"
                    style={{ width: `${v.total ? (100 * v.ac) / v.total : 0}%` }}
                  />
                </span>
                <span className="w-12 text-right text-paper-500">
                  {v.ac}/{v.total}
                </span>
              </li>
            ))}
        </ul>
        <h2 className="mt-8 text-sm font-medium text-paper-400">Recent submits</h2>
        <ul className="mt-3 space-y-1">
          {(progress?.attempts ?? []).slice(0, 20).map((a) => {
            const p = problems.find((x) => x.id === a.problemId);
            return (
              <li key={a.id}>
                <button
                  className="flex w-full items-center justify-between rounded-md px-2 py-1.5 text-left text-sm hover:bg-ink-850"
                  onClick={() => openProblem(a.problemId)}
                >
                  <span>
                    {p?.title ?? a.problemId}{" "}
                    <span className="text-paper-500">{a.language}</span>
                  </span>
                  <span className={a.passed ? "text-easy" : "text-hard"}>{a.verdict}</span>
                </button>
              </li>
            );
          })}
        </ul>
      </div>
    </div>
  );
}

