import { runConfiguredPickerCommand } from "./shared";

export async function searchFilesConfigured(): Promise<void> {
  await runConfiguredPickerCommand("Binocular Files (Configured Folders)", ["files"]);
}
