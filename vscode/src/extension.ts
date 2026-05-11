import * as vscode from "vscode";
import { runCustomCommand } from "./commands/customCommands";
import { searchContent } from "./commands/searchContent";
import { searchContentConfigured } from "./commands/searchContentConfigured";
import { searchContentWithSelection } from "./commands/searchContentWithSelection";
import { searchContentWithSelectionConfigured } from "./commands/searchContentWithSelectionConfigured";
import { searchDirectories } from "./commands/searchDirectories";
import { searchDirectoriesConfigured } from "./commands/searchDirectoriesConfigured";
import { searchFiles } from "./commands/searchFiles";
import { searchFilesConfigured } from "./commands/searchFilesConfigured";
import { searchPath } from "./commands/searchPath";
import { searchPathConfigured } from "./commands/searchPathConfigured";

export function activate(context: vscode.ExtensionContext): void {
  context.subscriptions.push(
    vscode.commands.registerCommand("binocular.searchPath", searchPath),
    vscode.commands.registerCommand("binocular.searchPathConfigured", searchPathConfigured),
    vscode.commands.registerCommand("binocular.searchFiles", searchFiles),
    vscode.commands.registerCommand("binocular.searchFilesConfigured", searchFilesConfigured),
    vscode.commands.registerCommand("binocular.searchContent", searchContent),
    vscode.commands.registerCommand("binocular.searchContentConfigured", searchContentConfigured),
    vscode.commands.registerCommand(
      "binocular.searchContentWithSelection",
      searchContentWithSelection,
    ),
    vscode.commands.registerCommand(
      "binocular.searchContentWithSelectionConfigured",
      searchContentWithSelectionConfigured,
    ),
    vscode.commands.registerCommand("binocular.searchDirectories", searchDirectories),
    vscode.commands.registerCommand(
      "binocular.searchDirectoriesConfigured",
      searchDirectoriesConfigured,
    ),
    vscode.commands.registerCommand("binocular.runCustomCommand", runCustomCommand),
  );
}

export function deactivate(): void {}
