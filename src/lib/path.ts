import { problems, problemsForUnit, units } from "./content";
import type { Problem, Unit } from "./types";

export function unitUnlocked(unit: Unit, accepted: Set<string>): boolean {
  if (unit.week <= 1) return true;
  const prev = units.find((u) => u.week === unit.week - 1);
  if (!prev) return true;
  return prev.coreIds.every((id) => accepted.has(id));
}

export function nextProblem(states: { problemId: string; status: string }[]): Problem | undefined {
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
  const leftover = problems.find((p) => !accepted.has(p.id));
  return leftover;
}