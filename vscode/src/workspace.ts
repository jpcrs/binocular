import * as path from "node:path";
import * as vscode from "vscode";
import { CustomCommandCwd, loadConfig } from "./config";

export interface PlaceholderContext {
  workspaceFolder: string;
  workspaceFolderName: string;
  file: string;
  relativeFile: string;
  selectedText: string;
}

export function getConfiguredSearchPaths(): string[] {
  return loadConfig().searchPaths;
}

export function getWorkspaceFoldersOrWarn(): readonly vscode.WorkspaceFolder[] | undefined {
  const folders = vscode.workspace.workspaceFolders;
  if (!folders || folders.length === 0) {
    vscode.window.showInformationMessage("Binocular requires an open workspace folder.");
    return undefined;
  }

  return folders;
}

export function buildWorkspaceLocationArgs(): string[] | undefined {
  const folders = getWorkspaceFoldersOrWarn();
  if (!folders) {
    return undefined;
  }

  return folders.flatMap((folder) => ["-l", folder.uri.fsPath]);
}

export function buildConfiguredLocationArgs(): string[] | undefined {
  const configuredPaths = getConfiguredSearchPaths();
  if (configuredPaths.length === 0) {
    vscode.window.showInformationMessage(
      "Binocular has no configured search paths. Set binocular.searchPaths in settings.",
    );
    return undefined;
  }

  return configuredPaths.flatMap((folderPath) => ["-l", folderPath]);
}

export function getPickerCwd(): vscode.Uri | undefined {
  return getActiveWorkspaceFolder()?.uri ?? vscode.workspace.workspaceFolders?.[0]?.uri;
}

export function getConfiguredPickerCwd(): vscode.Uri | undefined {
  const configuredPaths = getConfiguredSearchPaths();
  if (configuredPaths.length === 0) {
    vscode.window.showInformationMessage(
      "Binocular has no configured search paths. Set binocular.searchPaths in settings.",
    );
    return undefined;
  }

  return vscode.Uri.file(configuredPaths[0]);
}

export function getSelectedText(): string | undefined {
  const editor = vscode.window.activeTextEditor;
  if (!editor || editor.selection.isEmpty) {
    return undefined;
  }

  const text = editor.document.getText(editor.selection).trim();
  return text.length > 0 ? text : undefined;
}

export function buildPlaceholderContext(): PlaceholderContext {
  const editor = vscode.window.activeTextEditor;
  const workspaceFolder = editor
    ? vscode.workspace.getWorkspaceFolder(editor.document.uri)
    : vscode.workspace.workspaceFolders?.[0];
  const filePath = editor?.document.uri.fsPath ?? "";

  return {
    workspaceFolder: workspaceFolder?.uri.fsPath ?? "",
    workspaceFolderName: workspaceFolder?.name ?? "",
    file: filePath,
    relativeFile:
      workspaceFolder && filePath
        ? path.relative(workspaceFolder.uri.fsPath, filePath)
        : "",
    selectedText: getSelectedText() ?? "",
  };
}

export function resolveCommandCwd(cwd: CustomCommandCwd): vscode.Uri | undefined {
  if (cwd === "activeFileDir") {
    const filePath = vscode.window.activeTextEditor?.document.uri.fsPath;
    if (filePath) {
      return vscode.Uri.file(path.dirname(filePath));
    }
  }

  if (cwd === "firstWorkspace") {
    return vscode.workspace.workspaceFolders?.[0]?.uri;
  }

  return getActiveWorkspaceFolder()?.uri ?? vscode.workspace.workspaceFolders?.[0]?.uri;
}

function getActiveWorkspaceFolder(): vscode.WorkspaceFolder | undefined {
  const editor = vscode.window.activeTextEditor;
  if (!editor) {
    return undefined;
  }

  return vscode.workspace.getWorkspaceFolder(editor.document.uri);
}
