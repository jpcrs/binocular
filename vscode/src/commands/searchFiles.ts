import { runPickerCommand } from "./shared";

export async function searchFiles(): Promise<void> {
  await runPickerCommand("Binocular Files", ["files"]);
}
