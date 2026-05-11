import { runPickerCommand } from "./shared";

export async function searchContent(): Promise<void> {
  await runPickerCommand("Binocular Content", ["content"]);
}
