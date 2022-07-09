/* eslint-disable @typescript-eslint/naming-convention */
import * as vscode from 'vscode';
import { getTempFile } from "./folderUtils";
import { Command, Config } from "./types";
import * as os from 'os';

/**
 * List of handler commands that will be used to parse the shell command with the right syntax.
 */
const handlers: { [key: string]: (cfg: Config) => string } = {
    '{pwd}': getOsPwd,
    '{workspaceFolders}': getWorkspaceFolders,
    '{configuredFolders}': getConfiguredFolders,
    '{workspaceFoldersLineBreak}': getWorkspaceFoldersWithLineBreak,
    '{sedRemoveGitFromString}': sedRemoveGit,
    '{sedReplaceSkipDelimiter}': sedSkipDelimiter,
};

/**
 * Parses the command and replaces the placeholders.
 * @param cmd Command that will be executed by the terminal, with the placeholders
 * @param cfg Config that has to be used to access some configuration values, like the configured folders.
 * @returns Final command that will be executed by the terminal.
 */
export function parseCommand(cmd: Command, cfg: Config): string {
    let shellCommand = cmd.shellCommand;
    Object.entries(handlers).forEach(([key, handler]) => {
        shellCommand = shellCommand.replace(key, handler(cfg));
    });

    if (cfg.externalTerminal) {
        return `${cfg.externalTerminalCustomCommand.replaceAll("#", `${shellCommand} ${tee()} ${getTempFile(cmd.outputFile, cfg)}`)}`;
    }

    return `${shellCommand} ${tee()} ${getTempFile(cmd.outputFile, cfg)}`;
}

/**
 * (pwd).path on windows (Powershell syntax)
 * $(pwd) on linux and macOS.
 * @returns The PWD command for the current operating system.
 */
function getOsPwd(): string {
    switch (os.platform()) {
        case 'win32':
            return '(pwd).path';
        default:
            return '$(pwd)';
    }
}

/**
 * @param cfg Config, used to get the path to the temporary folder/file.
 * @returns All the folders, separated by space.
 */
function getConfiguredFolders(cfg: Config): string {
    return cfg.additionalFolders?.join(' ') ?? "";
}

/**
 * @returns All the folders open in vscode, separated by space.
 */
function getWorkspaceFolders(): string {
    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (!workspaceFolders) {
        return '';
    }
    return workspaceFolders.map(folder => folder.uri.fsPath).join(' ');
}

/**
 * @returns All the folders open in vscode, separated by a linebreak, so it can be interpreted by fzf.
 * TODO: Change the fzf command to use whitespace as separator, so this function can be removed.
 */
function getWorkspaceFoldersWithLineBreak(): string {
    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (!workspaceFolders) {
        return '';
    }
    if (os.platform() === 'win32') {
        return `\"${workspaceFolders.map(folder => folder.uri.fsPath).join("\" \"")}\"`;
    }

    return workspaceFolders.map(folder => folder.uri.fsPath).join('\\\\n');
}

function tee(): string {
    switch(os.platform()) {
        case 'win32':
            return '| out-file -encoding ASCII';
        default:
            return '>';
    }
}

function sedRemoveGit(): string {
    switch(os.platform()) {
        case 'win32':
            return `sed 's/\\\\.git\\\\//g'`;
        default:
            return `sed 's/\\/.git//g'`;
    }
}
function sedSkipDelimiter(): string {
    switch(os.platform()) {
        case 'win32':
            return `sed 's/:/::/2g'`;
        default:
            return `sed 's/:/::/g'`;
    }
}