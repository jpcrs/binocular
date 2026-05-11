import { getSelectedText } from "../workspace";
import { runPickerCommand } from "./shared";

export async function searchContentWithSelection(): Promise<void> {
  await runPickerCommand("Binocular Content", ["content"], getSelectedText());
}
