import { runConfiguredPickerCommand } from "./shared";

export async function searchContentConfigured(): Promise<void> {
  await runConfiguredPickerCommand("Binocular Content (Configured Folders)", ["content"]);
}
