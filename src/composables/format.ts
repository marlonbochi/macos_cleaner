import { ref } from "vue";

/**
 * Locale used for numbers and dates. Reactive, so templates re-render when the language changes.
 */
const formatLocale = ref("en-US");

export function setFormatLocale(locale: string): void {
  formatLocale.value = locale;
}

const UNITS = ["B", "KB", "MB", "GB", "TB", "PB"] as const;

/** Human-readable size in base 1000, like Finder (e.g. "1.5 GB" / "1,5 GB"). */
export function formatBytes(bytes: number, locale = formatLocale.value): string {
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

export function formatCount(value: number, locale = formatLocale.value): string {
  return new Intl.NumberFormat(locale).format(value);
}

/** Formats a Unix timestamp (seconds) as a short date, or "—" when absent. */
export function formatDate(
  epochSeconds: number | null | undefined,
  locale = formatLocale.value,
): string {
  if (epochSeconds === null || epochSeconds === undefined) return "—";
  return new Intl.DateTimeFormat(locale, { dateStyle: "short" }).format(
    new Date(epochSeconds * 1000),
  );
}

export function formatDateTime(
  epochSeconds: number | null | undefined,
  locale = formatLocale.value,
): string {
  if (epochSeconds === null || epochSeconds === undefined) return "—";
  return new Intl.DateTimeFormat(locale, { dateStyle: "short", timeStyle: "short" }).format(
    new Date(epochSeconds * 1000),
  );
}

/** Shortens a path by replacing the home directory with "~". */
export function shortenPath(path: string, home: string | null | undefined): string {
  if (!home) return path;
  if (path === home) return "~";
  const separator = path.charAt(home.length);
  return path.startsWith(home) && (separator === "/" || separator === "\\")
    ? `~${path.slice(home.length)}`
    : path;
}

/** `C:\` style drive roots (Windows). */
const DRIVE_ROOT = /^[A-Za-z]:[\\/]?$/;

/** Parent directory of an absolute path, POSIX (`/`) or Windows (`C:\…`). The parent of a
 * top-level entry is the root (`/` or the drive). */
export function parentPath(path: string): string {
  if (DRIVE_ROOT.test(path)) return path;
  const windows = /^[A-Za-z]:[\\/]/.test(path);
  const trimmed = path.length > 1 ? path.replace(/[\\/]+$/, "") : path;
  const index = Math.max(trimmed.lastIndexOf("/"), windows ? trimmed.lastIndexOf("\\") : -1);
  if (windows) return index <= 2 ? trimmed.slice(0, 2) + "\\" : trimmed.slice(0, index);
  return index <= 0 ? "/" : trimmed.slice(0, index);
}
