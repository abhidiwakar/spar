import type { Problem, Unit } from "./types";

const unitModules = import.meta.glob("../../content/units.json", { eager: true });
const problemModules = import.meta.glob("../../content/problems/*.json", { eager: true });

function unwrap<T>(mod: unknown): T {
  if (mod && typeof mod === "object" && "default" in (mod as object)) {
    return (mod as { default: T }).default;
  }
  return mod as T;
}

export const units: Unit[] = unwrap<Unit[]>(Object.values(unitModules)[0]).sort(
  (a, b) => a.week - b.week,
);

export const problems: Problem[] = Object.values(problemModules)
  .map((m) => unwrap<Problem>(m))
  .sort((a, b) => a.unitId.localeCompare(b.unitId) || a.order - b.order);

export const problemById = new Map(problems.map((p) => [p.id, p]));

export function unitById(id: string): Unit | undefined {
  return units.find((u) => u.id === id);
}

export function problemsForUnit(unit: Unit): Problem[] {
  const ids = [...unit.coreIds, ...unit.stretchIds];
  return ids.map((id) => problemById.get(id)).filter((p): p is Problem => Boolean(p));
}
