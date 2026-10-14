const UNITS = ["B", "KB", "MB", "GB", "TB", "PB"] as const;

/** Human-readable size in base 1000, like Finder (e.g. "1,5 GB"). */
export function formatBytes(bytes: number, locale = "pt-BR"): string {
  if (!Number.isFinite(bytes) || bytes < 0) return "—";
  if (bytes < 1000) return `${bytes} B`;
  let value = bytes;
  let unit = 0;
  while (value >= 1000 && unit < UNITS.length - 1) {
    value /= 1000;
    unit += 1;
  }
  const number = new Intl.NumberFormat(locale, {
    minimumFractionDigits: 1,
    maximumFractionDigits: 1,
  }).format(value);
  return `${number} ${UNITS[unit]}`;
}

export function formatCount(value: number, locale = "pt-BR"): string {
  return new Intl.NumberFormat(locale).format(value);
}

/** Formats a Unix timestamp (seconds) as a short date, or "—" when absent. */
export function formatDate(epochSeconds: number | null | undefined, locale = "pt-BR"): string {
  if (epochSeconds === null || epochSeconds === undefined) return "—";
  return new Intl.DateTimeFormat(locale, { dateStyle: "short" }).format(
    new Date(epochSeconds * 1000),
  );
}

export function formatDateTime(epochSeconds: number | null | undefined, locale = "pt-BR"): string {
  if (epochSeconds === null || epochSeconds === undefined) return "—";
  return new Intl.DateTimeFormat(locale, { dateStyle: "short", timeStyle: "short" }).format(
    new Date(epochSeconds * 1000),
  );
}

/** Shortens a path by replacing the home directory with "~". */
export function shortenPath(path: string, home: string | null | undefined): string {
  if (!home) return path;
  if (path === home) return "~";
  return path.startsWith(`${home}/`) ? `~${path.slice(home.length)}` : path;
}

/** Parent directory of an absolute POSIX path ("/" for top-level entries). */
export function parentPath(path: string): string {
  const trimmed = path.length > 1 ? path.replace(/\/+$/, "") : path;
  const index = trimmed.lastIndexOf("/");
  return index <= 0 ? "/" : trimmed.slice(0, index);
}
