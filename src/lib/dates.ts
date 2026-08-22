/** Local calendar date as YYYY-MM-DD. Matches Rust `chrono::Local` used for XP. */
export function localDateString(d = new Date()): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
}

export function parseLocalDate(iso: string): Date {
  const [y, m, d] = iso.split("-").map(Number);
  return new Date(y, (m || 1) - 1, d || 1);
}

/** `n` local dates ending on `end` (inclusive). Prefer backend `progress.today`. */
export function lastNLocalDates(n: number, end = localDateString()): string[] {
  const last = parseLocalDate(end);
  const out: string[] = [];
  for (let i = n - 1; i >= 0; i--) {
    out.push(
      localDateString(new Date(last.getFullYear(), last.getMonth(), last.getDate() - i)),
    );
  }
  return out;
}
