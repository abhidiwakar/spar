import { problemsForUnit, sortedProblems, sortedUnits } from "./content";
import type { Problem, Unit } from "./types";

export function unitUnlocked(unit: Unit, units: Unit[], accepted: Set<string>): boolean {
  if (unit.week <= 1) return true;
  const prev = units.find((u) => u.week === unit.week - 1);
  if (!prev) return true;
  return prev.coreIds.every((id) => accepted.has(id));
}

export function nextProblem(
  units: Unit[],
  problems: Problem[],
  states: { problemId: string; status: string }[],
): Problem | undefined {
  const accepted = new Set(states.filter((s) => s.status === "accepted").map((s) => s.problemId));
  const orderedUnits = sortedUnits(units);
  const orderedProblems = sortedProblems(problems);
  for (const unit of orderedUnits) {
    if (!unitUnlocked(unit, orderedUnits, accepted)) continue;
    const list = problemsForUnit(unit, orderedProblems);
    const undone = list.find((p) => p.kind === "core" && !accepted.has(p.id));
    if (undone) return undone;
  }
  for (const unit of orderedUnits) {
    const stretch = problemsForUnit(unit, orderedProblems).find(
      (p) => p.kind === "stretch" && !accepted.has(p.id),
    );
    if (stretch) return stretch;
  }
  return orderedProblems.find((p) => !accepted.has(p.id));
}
