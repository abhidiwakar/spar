import type { Problem, Unit } from "./types";

export type CatalogSnapshot = {
  units: Unit[];
  problems: Problem[];
};

export function sortedUnits(units: Unit[]): Unit[] {
  return [...units].sort((a, b) => a.week - b.week);
}

export function sortedProblems(problems: Problem[]): Problem[] {
  return [...problems].sort((a, b) => a.unitId.localeCompare(b.unitId) || a.order - b.order);
}

export function problemByIdMap(problems: Problem[]): Map<string, Problem> {
  return new Map(problems.map((p) => [p.id, p]));
}

export function unitById(units: Unit[], id: string): Unit | undefined {
  return units.find((u) => u.id === id);
}

export function problemsForUnit(unit: Unit, problems: Problem[]): Problem[] {
  const byId = problemByIdMap(problems);
  const ids = [...unit.coreIds, ...unit.stretchIds];
  return ids.map((id) => byId.get(id)).filter((p): p is Problem => Boolean(p));
}
