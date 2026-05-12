import * as vscode from "vscode";
import { ensureBinocularBinary } from "../binary";
import { loadConfig } from "../config";
import { openSelectionRecords, parseSelectionOutput } from "../output";
import { launchPickerTerminal } from "../terminal";
import {
  buildConfiguredLocationArgs,
  buildWorkspaceLocationArgs,
  getConfiguredPickerCwd,
  getPickerCwd,
} from "../workspace";

export async function runPickerCommand(
  name: string,
  optionArgs: string[],
  query?: string,
): Promise<void> {
  const previousEditor = vscode.window.activeTextEditor;
  const binaryPath = await ensureBinocularBinary();
  if (!binaryPath) {
    return;
  }

  const locationArgs = buildWorkspaceLocationArgs();
  if (!locationArgs) {
    return;
  }

  const cwd = getPickerCwd();
  if (!cwd) {
    return;
  }

  const output = await launchPickerTerminal({
    name,
    command: binaryPath,
    globalArgs: ["--output-format", "jsonl"],
    args: [
      ...optionArgs,
      ...(loadConfig().useExact ? ["--exact"] : []),
      ...locationArgs,
      ...(query ? [query] : []),
    ],
    cwd,
  });

  if (!output || output.trim().length === 0) {
    await restorePreviousEditorFocus(previousEditor);
    return;
  }

  await openSelectionRecords(parseSelectionOutput(output));
}

export async function runConfiguredPickerCommand(
  name: string,
  optionArgs: string[],
  query?: string,
): Promise<void> {
  const previousEditor = vscode.window.activeTextEditor;
  const binaryPath = await ensureBinocularBinary();
  if (!binaryPath) {
    return;
  }

  const locationArgs = buildConfiguredLocationArgs();
  if (!locationArgs) {
    return;
  }

  const cwd = getConfiguredPickerCwd();
  if (!cwd) {
    return;
  }

  const output = await launchPickerTerminal({
    name,
    command: binaryPath,
    globalArgs: ["--output-format", "jsonl"],
    args: [
      ...optionArgs,
      ...(loadConfig().useExact ? ["--exact"] : []),
      ...locationArgs,
      ...(query ? [query] : []),
    ],
    cwd,
  });

  if (!output || output.trim().length === 0) {
    await restorePreviousEditorFocus(previousEditor);
    return;
  }

  await openSelectionRecords(parseSelectionOutput(output));
}

async function restorePreviousEditorFocus(
  previousEditor: vscode.TextEditor | undefined,
): Promise<void> {
  if (!previousEditor) {
    await vscode.commands.executeCommand("workbench.action.focusActiveEditorGroup");
    return;
  }

  try {
    await vscode.window.showTextDocument(previousEditor.document, {
      viewColumn: previousEditor.viewColumn,
      preview: false,
      preserveFocus: false,
      selection: previousEditor.selection,
    });
  } catch {
    await vscode.commands.executeCommand("workbench.action.focusActiveEditorGroup");
  }
}
