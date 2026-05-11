import { runPickerCommand } from "./shared";

export async function searchPath(): Promise<void> {
  await runPickerCommand("Binocular Paths", ["path"]);
}
