import { execFile } from "node:child_process";
import { promisify } from "node:util";
import * as vscode from "vscode";
import { loadConfig } from "./config";

const execFileAsync = promisify(execFile);
const DEFAULT_BINARY = "binocular";

interface BinaryCheckResult {
  path: string;
  version?: string;
}

export async function ensureBinocularBinary(): Promise<string | undefined> {
  const configuredPath = loadConfig().binaryPath;
  const binaryPath = configuredPath ?? DEFAULT_BINARY;

  let versionOutput: string;
  try {
    const { stdout } = await execFileAsync(binaryPath, ["--version"]);
    versionOutput = stdout;
  } catch {
    const message = configuredPath
      ? `Could not execute binocular from configured path: ${configuredPath}`
      : "Could not find `binocular` on PATH.";
    vscode.window.showErrorMessage(`${message} Set binocular.binaryPath or install the binary.`);
    return undefined;
  }

  const version = parseVersion(versionOutput);
  if (!version) {
    vscode.window.showErrorMessage(
      `Could not determine binocular version from output: "${versionOutput.trim()}"`
    );
    return undefined;
  }

  const minVersion = getMinVersionFromManifest();
  if (minVersion && isVersionLessThan(version, minVersion)) {
    const message = `Binocular TUI ${version} is installed, but this extension requires ${minVersion} or newer.`;
    const updateAction = "Open Releases";
    vscode.window.showWarningMessage(message, updateAction).then((choice) => {
      if (choice === updateAction) {
        vscode.env.openExternal(
          vscode.Uri.parse("https://github.com/jpcrs/Binocular/releases")
        );
      }
    });
    // We still return the binary path so the user can continue if they want,
    // but features may be broken.
  }

  return (await resolveExecutablePath(binaryPath)) ?? binaryPath;
}

async function resolveExecutablePath(command: string): Promise<string | undefined> {
  if (looksLikePath(command)) {
    return command;
  }

  try {
    const resolver = process.platform === "win32" ? "where.exe" : "which";
    const { stdout } = await execFileAsync(resolver, [command]);
    return stdout
      .split(/\r?\n/)
      .map((line) => line.trim())
      .find((line) => line.length > 0);
  } catch {
    return undefined;
  }
}

function looksLikePath(command: string): boolean {
  return /[\\/]/.test(command) || /^[A-Za-z]:/.test(command);
}

function parseVersion(output: string): string | undefined {
  // Expected formats:
  // "binocular 0.2.3"
  // "binocular 0.2.3-abc123"
  // "0.2.3"
  const match = output.match(/(\d+\.\d+\.\d+)/);
  return match?.[1];
}

function isVersionLessThan(a: string, b: string): boolean {
  const parse = (v: string) => v.split(".").map((n) => parseInt(n, 10));
  const av = parse(a);
  const bv = parse(b);
  for (let i = 0; i < Math.max(av.length, bv.length); i++) {
    const an = av[i] ?? 0;
    const bn = bv[i] ?? 0;
    if (an < bn) return true;
    if (an > bn) return false;
  }
  return false;
}

/**
 * Reads the minimum required TUI version from the extension's package.json
 * under the `binocular.tui.minVersion` field.
 */
function getMinVersionFromManifest(): string | undefined {
  try {
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    const manifest = require("../../package.json");
    return manifest?.binocular?.tui?.minVersion;
  } catch {
    return undefined;
  }
}
