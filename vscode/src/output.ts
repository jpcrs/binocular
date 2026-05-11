import * as vscode from "vscode";

export type SelectionRecord =
  | { kind: "path"; path: string }
  | { kind: "grep"; path: string; line: number; column?: number }
  | { kind: "preview_location"; path: string; line: number; column: number }
  | { kind: "stdin"; text: string };

export function parseSelectionOutput(raw: string): SelectionRecord[] {
  return raw
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0)
    .flatMap((line) => {
      try {
        const parsed = JSON.parse(line) as unknown;
        return isSelectionRecord(parsed) ? [parsed] : [];
      } catch {
        return [];
      }
    });
}

export async function openSelectionRecords(records: SelectionRecord[]): Promise<void> {
  for (const record of records) {
    if (record.kind === "stdin") {
      continue;
    }

    try {
      const document = await vscode.workspace.openTextDocument(vscode.Uri.file(record.path));
      const editor = await vscode.window.showTextDocument(document, {
        viewColumn: vscode.ViewColumn.Active,
        preview: false,
      });

      if (record.kind !== "path") {
        const line = Math.max(record.line - 1, 0);
        const column = Math.max((record.column ?? 1) - 1, 0);
        const position = new vscode.Position(line, column);
        const range = new vscode.Range(position, position);
        editor.selection = new vscode.Selection(position, position);
        editor.revealRange(range, vscode.TextEditorRevealType.InCenter);
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      vscode.window.showErrorMessage(`Failed to open selection: ${message}`);
    }
  }
}

function isSelectionRecord(value: unknown): value is SelectionRecord {
  if (!value || typeof value !== "object") {
    return false;
  }

  const record = value as Record<string, unknown>;
  if (record.kind === "path") {
    return typeof record.path === "string";
  }
  if (record.kind === "stdin") {
    return typeof record.text === "string";
  }
  if (record.kind === "grep") {
    return (
      typeof record.path === "string" &&
      typeof record.line === "number" &&
      (record.column === undefined || typeof record.column === "number")
    );
  }
  if (record.kind === "preview_location") {
    return (
      typeof record.path === "string" &&
      typeof record.line === "number" &&
      typeof record.column === "number"
    );
  }

  return false;
}
