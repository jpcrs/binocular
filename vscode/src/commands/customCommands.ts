import * as vscode from "vscode";
import { CustomCommandConfig, loadConfig } from "../config";
import { expandArgs, expandEnv } from "../placeholders";
import { launchCustomTerminal } from "../terminal";
import { buildPlaceholderContext, resolveCommandCwd } from "../workspace";

export async function runCustomCommand(selector?: string): Promise<void> {
  const commands = loadConfig().customCommands;
  if (commands.length === 0) {
    vscode.window.showInformationMessage("No Binocular custom commands are configured.");
    return;
  }

  const command = selector
    ? findCustomCommand(commands, selector)
    : await pickCustomCommand(commands);

  if (!command) {
    if (selector) {
      vscode.window.showErrorMessage(`No Binocular custom command matches: ${selector}`);
    }
    return;
  }

  const cwd = resolveCommandCwd(command.cwd);
  if (!cwd) {
    vscode.window.showErrorMessage("Could not resolve a working directory for the custom command.");
    return;
  }

  const placeholderContext = buildPlaceholderContext();
  await launchCustomTerminal({
    name: `Binocular: ${command.title}`,
    command: command.command,
    args: expandArgs(command.args, placeholderContext),
    cwd,
    env: expandEnv(command.env, placeholderContext),
    useShell: command.useShell,
  });
}

async function pickCustomCommand(
  commands: CustomCommandConfig[],
): Promise<CustomCommandConfig | undefined> {
  const picked = await vscode.window.showQuickPick(
    commands.map((command) => ({
      label: command.title,
      description: command.command,
      detail: command.id,
      command,
    })),
    {
      placeHolder: "Choose a custom command to run",
    },
  );

  return picked?.command;
}

function findCustomCommand(
  commands: CustomCommandConfig[],
  selector: string,
): CustomCommandConfig | undefined {
  const normalizedSelector = selector.trim().toLowerCase();
  if (normalizedSelector.length === 0) {
    return undefined;
  }

  return commands.find((command) => {
    const candidates = [command.id, command.title, command.command]
      .filter((value): value is string => typeof value === "string")
      .map((value) => value.trim().toLowerCase());
    return candidates.includes(normalizedSelector);
  });
}
