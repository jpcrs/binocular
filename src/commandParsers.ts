import * as vscode from 'vscode';
import { getTempFile } from "./folderUtils";
import { Command, Config } from "./types";
import * as os from 'os';

/**
 * Parses the command and replaces the placeholders.
 * @param cmd Command that will be executed by the terminal, with the placeholders
 * @param cfg Config that has to be used to access some configuration values, like the configured folders.
 * @returns Final command that will be executed by the terminal.
 */
export function parseCommand(cmd: Command, cfg: Config): string {
    let shellCommand = cmd.shellCommand(cfg);
    shellCommand = shellCommand.replaceAll("{pwd}", getOsPwd());
    shellCommand = shellCommand.replaceAll("{workspaceFolders}", getWorkspaceFolders());
    shellCommand = shellCommand.replaceAll("{configuredFolders}", getConfiguredFolders(cfg));
    shellCommand = shellCommand.replaceAll("{workspaceFoldersLineBreak}", getWorkspaceFoldersWithLineBreak());
    shellCommand = shellCommand.replaceAll("{sedRemoveGit}", sedRemoveGit());
    shellCommand = shellCommand.replaceAll("{sedSkipDelimiter}", sedSkipDelimiter());

    if (cfg.externalTerminal) {
        return `${cfg.externalTerminalCustomCommand.replaceAll("#", `${shellCommand} ${tee()} ${getTempFile(cmd.outputFile, cfg)}`)}`;
    }

    return `${shellCommand} ${tee()} ${getTempFile(cmd.outputFile, cfg)}`;
}

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