import type { FileEntryDto } from "@/types/generated/FileEntryDto";
import type { PageDto } from "@/types/generated/PageDto";
import type { ScanSummaryDto } from "@/types/generated/ScanSummaryDto";

export function scanSummary(overrides: Partial<ScanSummaryDto> = {}): ScanSummaryDto {
  return {
    id: 1,
    roots: ["/Users/alice"],
    status: "Completed",
    minFileSize: 100_000_000,
    startedAt: 1_700_000_000,
    finishedAt: 1_700_000_060,
    filesSeen: 1000,
    bytesSeen: 50_000_000_000,
    inaccessibleCount: 0,
    error: null,
    incomplete: false,
    ...overrides,
  };
}

export function fileEntry(overrides: Partial<FileEntryDto> = {}): FileEntryDto {
  const path = overrides.path ?? "/Users/alice/Movies/video.mov";
  return {
    id: 1,
    path,
    name: path.split("/").at(-1) ?? path,
    kind: "File",
    sizeOnDisk: 5_000_000_000,
    apparentSize: 5_000_000_000,
    fileCount: 1,
    modifiedAt: 1_700_000_000,
    accessedAt: null,
    isProtected: false,
    ...overrides,
  };
}

export function page<T>(items: T[], total = items.length, offset = 0): PageDto<T> {
  return { items, total, offset, limit: 100 };
}
