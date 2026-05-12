import * as fs from "node:fs/promises";
import * as os from "node:os";
import * as path from "node:path";
import * as vscode from "vscode";

const PICKER_TERMINAL_NAME = "Binocular Picker";

interface CommandSpec {
  command: string;
  globalArgs?: string[];
  args?: string[];
  cwd?: vscode.Uri;
  env?: Record<string, string>;
  name: string;
  useShell?: boolean;
}

export async function launchPickerTerminal(spec: CommandSpec): Promise<string | undefined> {
  disposeTerminalByName(PICKER_TERMINAL_NAME);

  const outputFile = path.join(
    os.tmpdir(),
    `binocular-selection-${Date.now()}-${Math.random().toString(36).slice(2)}.jsonl`,
  );
  const terminal = createTerminal({
    ...spec,
    name: PICKER_TERMINAL_NAME,
    globalArgs: [...(spec.globalArgs ?? []), "--output-file", outputFile],
  });

  terminal.show(false);
  return waitForTerminalOutput(terminal, outputFile);
}

export async function launchCustomTerminal(spec: CommandSpec): Promise<void> {
  disposeTerminalByName(spec.name);
  const previousEditor = vscode.window.activeTextEditor;
  const commandLine = buildCommandLine(
    spec.command,
    getCommandArgs(spec),
    undefined,
    spec.useShell,
  );
  const terminal = createTerminal({
    ...spec,
    commandLine,
  });

  terminal.show(false);
  await waitForTerminalClose(terminal);
  await restorePreviousEditorFocus(previousEditor);
}

function createTerminal(
  spec: CommandSpec & { commandLine?: string },
): vscode.Terminal {
  if (!spec.useShell) {
    return vscode.window.createTerminal({
      name: spec.name,
      shellPath: spec.command,
      shellArgs: getCommandArgs(spec),
      cwd: spec.cwd,
      env: spec.env,
      location: vscode.TerminalLocation.Editor,
    });
  }

  const commandLine =
    spec.commandLine ?? buildCommandLine(spec.command, getCommandArgs(spec), undefined, true);
  const { shellPath, shellArgs } = buildShellInvocation(commandLine);
  return vscode.window.createTerminal({
    name: spec.name,
    shellPath,
    shellArgs,
    cwd: spec.cwd,
    env: spec.env,
    location: vscode.TerminalLocation.Editor,
  });
}

function getCommandArgs(spec: CommandSpec): string[] {
  return [...(spec.globalArgs ?? []), ...(spec.args ?? [])];
}

function disposeTerminalByName(name: string): void {
  const terminal = vscode.window.terminals.find((item) => item.name === name);
  terminal?.dispose();
}

function buildShellInvocation(commandLine: string): {
  shellPath: string;
  shellArgs: string[];
} {
  if (process.platform === "win32") {
    return {
      shellPath: "cmd.exe",
      shellArgs: ["/d", "/s", "/c", `"${commandLine}"`],
    };
  }

  return {
    shellPath: "sh",
    shellArgs: ["-c", commandLine],
  };
}

function buildCommandLine(
  command: string,
  args: string[],
  outputFile?: string,
  useShell = false,
): string {
  const parts = useShell ? [command, ...args] : [quoteShellArg(command), ...args.map(quoteShellArg)];
  const baseCommand = parts.join(" ");
  if (!outputFile) {
    return baseCommand;
  }

  return `${baseCommand} > ${quoteShellArg(outputFile)}`;
}

function quoteShellArg(value: string): string {
  if (process.platform === "win32") {
    return `"${value.replace(/"/g, '""')}"`;
  }

  return `'${value.replace(/'/g, `'"'"'`)}'`;
}

async function waitForTerminalOutput(
  terminal: vscode.Terminal,
  outputFile: string,
): Promise<string | undefined> {
  return new Promise((resolve) => {
    const closeListener = vscode.window.onDidCloseTerminal(async (closedTerminal) => {
      if (closedTerminal !== terminal) {
        return;
      }

      closeListener.dispose();
      resolve(await readAndDeleteFile(outputFile));
    });
  });
}

async function waitForTerminalClose(terminal: vscode.Terminal): Promise<void> {
  return new Promise((resolve) => {
    const closeListener = vscode.window.onDidCloseTerminal((closedTerminal) => {
      if (closedTerminal !== terminal) {
        return;
      }

      closeListener.dispose();
      resolve();
    });
  });
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

async function readAndDeleteFile(filePath: string): Promise<string | undefined> {
  try {
    const content = await fs.readFile(filePath, "utf8");
    await fs.rm(filePath, { force: true });
    return content;
  } catch {
    return undefined;
  }
}
