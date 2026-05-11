import { runPickerCommand } from "./shared";

export async function searchDirectories(): Promise<void> {
  await runPickerCommand("Binocular Directories", ["dirs"]);
}
