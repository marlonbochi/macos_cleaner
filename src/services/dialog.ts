import { open } from "@tauri-apps/plugin-dialog";

/** Opens the native folder picker. Returns the selected absolute paths (empty if cancelled). */
export async function pickFolders(): Promise<string[]> {
  const selection = await open({ directory: true, multiple: true });
  if (selection === null) return [];
  return Array.isArray(selection) ? selection : [selection];
}
