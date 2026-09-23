/**
 * Shared helpers for rendering GitHub/Airflow-style day-square run history
 * calendars, either across all Workflows or scoped to a single Workflow.
 */
export type HistoryEntry = { time: string; errored: boolean };
export type HistoryDay = { key: string; count: number; erroredLast: boolean };

const DAY_MS = 24 * 60 * 60 * 1000;

const dayKey = (d: Date) => d.toISOString().slice(0, 10);

export const levelFromCount = (count: number): number => {
  if (count === 0) return 0;
  if (count === 1) return 1;
  if (count <= 3) return 2;
  if (count <= 6) return 3;
  return 4;
};

// Binary appearance: an empty ring for no runs, a solid circle for any day a
// Workflow ran - green if the most recent run that day succeeded/is still
// going, red if it failed (still using `levelFromCount` to pick a shade).
export const GREEN_LEVEL_COLORS = [
  "bg-transparent border border-border",
  "bg-nexus-green/50 border border-nexus-green/60",
  "bg-nexus-green/70 border border-nexus-green/80",
  "bg-nexus-green/85 border border-nexus-green",
  "bg-nexus-green border border-nexus-green",
];

export const RED_LEVEL_COLORS = [
  "bg-transparent border border-border",
  "bg-nexus-red/50 border border-nexus-red/60",
  "bg-nexus-red/70 border border-nexus-red/80",
  "bg-nexus-red/85 border border-nexus-red",
  "bg-nexus-red border border-nexus-red",
];

export const colorForDay = (day: HistoryDay): string => {
  const level = levelFromCount(day.count);
  const palette = day.erroredLast ? RED_LEVEL_COLORS : GREEN_LEVEL_COLORS;
  // `level` is always 0-4, well within the fixed-length palette arrays.
  return palette[level] as string;
};

/** Human-readable label for a "YYYY-MM-DD" day key, e.g. "Sep 23". */
export const formatDayLabel = (key: string): string =>
  new Date(`${key}T00:00:00Z`).toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
    timeZone: "UTC",
  });

/** Build a fixed-width grid of days (oldest first) ending today, counting how many `entries` fall on each day. */
export const buildHistoryDays = (
  entries: HistoryEntry[],
  weeks: number,
): HistoryDay[] => {
  const counts = new Map<string, number>();
  const latest = new Map<string, HistoryEntry>();
  for (const entry of entries) {
    if (!entry.time) continue;
    const key = dayKey(new Date(entry.time));
    counts.set(key, (counts.get(key) ?? 0) + 1);
    const current = latest.get(key);
    if (!current || entry.time > current.time) {
      latest.set(key, entry);
    }
  }

  const today = new Date();
  today.setUTCHours(0, 0, 0, 0);
  // Anchor on the Sunday of the *current* week first, then walk back the
  // remaining weeks - guarantees "today" always falls inside the grid.
  // (Rewinding to Sunday after subtracting `totalDays` would instead shift
  // the whole window backwards and could cut off the most recent days.)
  const currentWeekStart = new Date(today);
  currentWeekStart.setUTCDate(
    currentWeekStart.getUTCDate() - currentWeekStart.getUTCDay(),
  );
  const start = new Date(currentWeekStart.getTime() - (weeks - 1) * 7 * DAY_MS);
  const totalDays = weeks * 7;

  return Array.from({ length: totalDays }, (_, i) => {
    const date = new Date(start.getTime() + i * DAY_MS);
    const key = dayKey(date);
    return {
      key,
      count: counts.get(key) ?? 0,
      erroredLast: latest.get(key)?.errored ?? false,
    };
  });
};

export const toWeeks = (days: HistoryDay[]): HistoryDay[][] => {
  const result: HistoryDay[][] = [];
  for (let i = 0; i < days.length; i += 7) {
    result.push(days.slice(i, i + 7));
  }
  return result;
};
