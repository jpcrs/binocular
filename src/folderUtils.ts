import * as vscode from 'vscode';
import { Config } from './config';
import * as path from 'path';
import { tmpdir } from 'os';
import * as fs from 'fs' ;
import { EXTENSION_NAME } from './const';

/**
 * @param cfg Config, used to get the path to the temporary folder/file.
 * @returns All the folders, separated by space.
 */
export function getConfiguredFolders(cfg: Config): string {
    return cfg.additionalFolders?.join(' ') ?? "";
}

/**
 * @returns All the folders open in vscode, separated by space.
 */
export function getWorkspaceFolders(): string {
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
export function getWorkspaceFoldersWithLineBreak(): string {
    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (!workspaceFolders) {
        return '';
    }
    return workspaceFolders.map(folder => folder.uri.fsPath).join('\\\\n');
}

/**
 * 
 * @param fileName Name of the file to be created.
 * @param cfg Config, used to get the guid for this vscode instance.
 * @returns The temporary folder for new files being watched by filewatcher.
 */
export function getTempFile(fileName: string, cfg: Config): string {
    return `${tmpdir()}${path.sep}${EXTENSION_NAME}${path.sep}${fileName}-${cfg.guid}`;
}

export function getTempFolder(fileName: string): string {
    return `${tmpdir()}${path.sep}${EXTENSION_NAME}${path.sep}${fileName}`;
}

/**
 * Creates the temporary directory used by the plugin.
 */
export function createTempDir(): void {
    const tempDir = `${tmpdir()}${path.sep}${EXTENSION_NAME}`;
    if (!fs.existsSync(tempDir)) {
        fs.mkdirSync(tempDir);
    }
}   