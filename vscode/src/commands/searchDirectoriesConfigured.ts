import { runConfiguredPickerCommand } from "./shared";

export async function searchDirectoriesConfigured(): Promise<void> {
  await runConfiguredPickerCommand("Binocular Directories (Configured Folders)", ["dirs"]);
}
