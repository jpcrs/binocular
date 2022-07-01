import * as fs from 'fs' ;
import * as vscode from 'vscode';
import { Config } from './config';
import { createTempDir, getTempFile, getTempFolder } from './folderUtils';
import { ITerminal } from './terminal';

/**
 * Structure to register new file watchers.
 */
export interface FileHandler {
    /**
     * Name of the file to watch.
     */
    fileName: string,

    /**
     * Function to call when the file is changed.
     */
    handler: (data: string) => void,
}

/**
 * Register file watchers for each one of our commands.
 * @param fileHandlers Structure to register new file watchers.
 * @param cfg Config, used to get the path to the temporary folder/file.
 * @param terminal Terminal object, so we can dispose it after the command is executed.
 */
export function registerFileWatchers(fileHandlers: FileHandler[], cfg: Config, terminal: ITerminal) {
    createTempDir();
    fileHandlers.forEach(item => {
        fs.writeFileSync(`${getTempFile(item.fileName, cfg)}`, '');
        fs.watch(`${getTempFile(item.fileName, cfg)}`, (x, y) => fileWatcherWrapper(x, y, terminal, item.handler));
    });
}

/**
 * Default wrapper with common functionality for each one of our watchers
 */
function fileWatcherWrapper(event: fs.WatchEventType, fileName: string, terminal: ITerminal, command: (data: string, terminal: ITerminal) => void): fs.WatchListener<string> {
    if (event !== "change") {
        return (x => x);
    }

    fs.readFile(`${getTempFolder(fileName)}`, { encoding: 'utf-8' }, (err, data) => {
        if (!data) {
            return;
        }
        command(data, terminal);
    });
    return (x => x);
}

/**
 * Open the files on vscode.
 * @param data Data that was inserted in the file. Hopefuly it's the output of rg/fd/fzf.
 * @param terminal Terminal to be disposed after the command is executed.
 */
export function openFile(data: string, terminal: ITerminal) {
    const filePaths = data.split('\n').filter(s => s !== '');
    filePaths.forEach(file => {
        vscode.window.showTextDocument(vscode.Uri.file(file), { preview: false });
    });
    terminal.dispose();
}

/**
 * Open the files on vscode. After that jumps to the line selected.
 * @param data Data that was inserted in the file.
 * @param terminal Terminal to be disposed after the command is executed.
 */
export function openFileAndJumpToLine(data: string, terminal: ITerminal) {
    const filePaths = data.split('\n').filter(s => s !== '');
    filePaths.forEach(file => {
        const fileInfo = file.split(':'); // [0] = file path, [1] = line number
        vscode.window.showTextDocument(vscode.Uri.file(fileInfo[0]), {
            selection: new vscode.Range(parseInt(fileInfo[1]) - 1, 0, parseInt(fileInfo[1]) - 1, 0), preview: false
        });
    });
    terminal.dispose();
}

/**
 * Add the folders to the workspace.
 * @param data Data that was inserted in the file.
 * @param terminal Terminal to be disposed after the command is executed.
 */
export function addFolderToWorkspace(data: string, terminal: ITerminal) {
    var existingWorkspaces = vscode.workspace.workspaceFolders?.map(x => x.uri.fsPath);
    const files = data.split('\n').filter(s => s !== '' && !existingWorkspaces?.includes(s)).map(x => ({
        uri: vscode.Uri.file(x),
        name: x.split('/').pop()
    }));

    vscode.workspace.updateWorkspaceFolders(vscode.workspace.workspaceFolders?.length ?? 0, null, ...files);
    terminal.dispose();
}

/**
 * Change to the folder, it'll reload the host/plugins and close all the other folders in the workspace.
 * @param data Data that was inserted in the file.
 * @param terminal Terminal to be disposed after the command is executed.
 */
export function changeToWorkspace(data: string, terminal: ITerminal) {
    const filePaths = data.split('\n').filter(s => s !== '');
    filePaths.forEach(file => {
        vscode.commands.executeCommand('vscode.openFolder', vscode.Uri.file(file));
    });
    terminal.dispose();
}

/**
 * Remove the folders from the workspace.
 * @param data Data that was inserted in the file.
 * @param terminal Terminal to be disposed after the command is executed.
 */
export function removeFromWorkspace(data: string, terminal: ITerminal) {
    const filePaths = data.split('\n').filter(s => s !== '');
    filePaths.forEach(async file => {
        const disposable = vscode.workspace.onDidChangeWorkspaceFolders(e => {
            const workspaceIndex = vscode.workspace.workspaceFolders?.findIndex(x => x.uri.fsPath === file);
            if (workspaceIndex !== undefined)
            {
                vscode.workspace.updateWorkspaceFolders(workspaceIndex, 1);
            }
            disposable.dispose();
        });

        const workspaceIndex = vscode.workspace.workspaceFolders?.findIndex(x => x.uri.fsPath === file);
        if (workspaceIndex !== undefined)
        {
            vscode.workspace.updateWorkspaceFolders(workspaceIndex, 1);
        }

        terminal.dispose();
    });
}