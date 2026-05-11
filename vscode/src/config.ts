import * as vscode from "vscode";

export const EXTENSION_NAMESPACE = "binocular";

export type CustomCommandCwd = "activeWorkspace" | "firstWorkspace" | "activeFileDir";

export interface CustomCommandConfig {
  id?: string;
  title: string;
  command: string;
  args: string[];
  cwd: CustomCommandCwd;
  env: Record<string, string>;
  useShell: boolean;
}

export interface ExtensionConfig {
  binaryPath: string | undefined;
  customCommands: CustomCommandConfig[];
  searchPaths: string[];
  useExact: boolean;
}

export function loadConfig(): ExtensionConfig {
  const config = vscode.workspace.getConfiguration(EXTENSION_NAMESPACE);
  const binaryPath = normalizeOptionalString(config.get<string>("binaryPath"));
  const rawCommands = config.get<unknown[]>("customCommands") ?? [];
  const rawSearchPaths = config.get<unknown[]>("searchPaths") ?? [];

  return {
    binaryPath,
    customCommands: rawCommands
      .map(normalizeCustomCommand)
      .filter((command): command is CustomCommandConfig => command !== undefined),
    searchPaths: rawSearchPaths
      .filter((item): item is string => typeof item === "string")
      .map((item) => item.trim())
      .filter((item) => item.length > 0),
    useExact: config.get<boolean>("useExact") ?? false,
  };
}

function normalizeCustomCommand(value: unknown): CustomCommandConfig | undefined {
  if (!value || typeof value !== "object") {
    return undefined;
  }

  const record = value as Record<string, unknown>;
  const id = normalizeOptionalString(record.id);
  const title = normalizeOptionalString(record.title);
  const command = normalizeOptionalString(record.command);

  if (!title || !command) {
    return undefined;
  }

  return {
    id,
    title,
    command,
    args: Array.isArray(record.args)
      ? record.args.filter((item): item is string => typeof item === "string")
      : [],
    cwd: normalizeCwd(record.cwd),
    env: normalizeEnv(record.env),
    useShell: record.useShell === true,
  };
}

function normalizeOptionalString(value: unknown): string | undefined {
  if (typeof value !== "string") {
    return undefined;
  }

  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : undefined;
}

function normalizeCwd(value: unknown): CustomCommandCwd {
  return value === "firstWorkspace" || value === "activeFileDir"
    ? value
    : "activeWorkspace";
}

function normalizeEnv(value: unknown): Record<string, string> {
  if (!value || typeof value !== "object") {
    return {};
  }

  return Object.fromEntries(
    Object.entries(value as Record<string, unknown>).flatMap(([key, item]) =>
      typeof item === "string" ? [[key, item]] : [],
    ),
  );
}
