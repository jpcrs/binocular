import { getSelectedText } from "../workspace";
import { runConfiguredPickerCommand } from "./shared";

export async function searchContentWithSelectionConfigured(): Promise<void> {
  await runConfiguredPickerCommand(
    "Binocular Content (Configured Folders)",
    ["content"],
    getSelectedText(),
  );
}
