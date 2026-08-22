import { problems } from "../../lib/content";
import { localDateString } from "../../lib/dates";
import { nextProblem } from "../../lib/path";
import { useApp } from "../../lib/store";

export function HomeScreen() {
  const progress = useApp((s) => s.progress);
  const openProblem = useApp((s) => s.openProblem);
  const setScreen = useApp((s) => s.setScreen);
  const next = nextProblem(progress?.problemStates ?? []);
  const accepted = new Set(
    (progress?.problemStates ?? []).filter((s) => s.status === "accepted").map((s) => s.problemId),
  );
  const goal = progress?.settings.dailyGoal ?? 1;
  const today = progress?.daily.find((d) => d.date === (progress.today ?? localDateString()));
  const goalMet = Boolean(today?.goalMet);

  return (
    <div className="mx-auto flex h-full max-w-3xl flex-col justify-center gap-10 px-6 py-10">
      <div>
        <p className="text-xs uppercase tracking-[0.2em] text-gold-400">DSA gym</p>
        <h1 className="mt-2 font-serif text-4xl leading-tight">Solve the next one.</h1>
        <p className="mt-3 max-w-xl text-paper-400">
          A 12-week path of medium-first problems. Arrays, trees, graphs, and DP in the order they
          build on each other. Write Python or JavaScript, run tests, submit.
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

