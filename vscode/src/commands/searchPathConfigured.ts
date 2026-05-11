import { runConfiguredPickerCommand } from "./shared";

export async function searchPathConfigured(): Promise<void> {
  await runConfiguredPickerCommand("Binocular Paths (Configured Folders)", ["path"]);
}
