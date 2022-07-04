import * as fs from 'fs' ;
import * as vscode from 'vscode';
import { getTempFile } from './folderUtils';
import { Command, Config, ITerminal } from './types';

/**
 * Register file watchers for each one of our commands.
 * @param commands List of commands to be registered
 * @param cfg Config, used to get the path to the temporary folder/file.
 * @param terminal Terminal object, so we can dispose it after the command is executed.
 */
export function registerFileWatchers(commands: Command[], cfg: Config, terminal: ITerminal) {
    commands.forEach(command => {
        fs.writeFileSync(`${getTempFile(command.outputFile, cfg)}`, '');
        fs.watch(`${getTempFile(command.outputFile, cfg)}`, (x, y) => fileWatcherWrapper(x, command, cfg, terminal));
    });
}

/**
 * Default wrapper with common functionality for each one of our watchers
 */
function fileWatcherWrapper(event: fs.WatchEventType, command: Command, config: Config, terminal: ITerminal): fs.WatchListener<string> {
    if (event !== "change") {
        return (x => x);
    }

    fs.readFile(`${getTempFile(command.outputFile, config)}`, { encoding: 'utf-8' }, (err, data) => {
        if (!data) {
            return;
        }
        command.handler(data, command, terminal);
    });
    return (x => x);
}


export function executeCustomCommand(data: string, command: Command, terminal: ITerminal) {
    let scriptContent = fs.readFileSync(command.scriptPath!, {encoding:'utf8', flag:'r'});
    var func = new Function(scriptContent)();
    func(data, vscode, terminal);
}

/**
 * Open the files on vscode.
 * @param data Data that was inserted in the file. Hopefuly it's the output of rg/fd/fzf.
 * @param terminal Terminal to be disposed after the command is executed.
 */
export function openFile(data: string, command: Command, terminal: ITerminal) {
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
export function openFileAndJumpToLine(data: string, command: Command, terminal: ITerminal) {
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
export function addFolderToWorkspace(data: string, command: Command, terminal: ITerminal) {
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
export function changeToWorkspace(data: string, command: Command, terminal: ITerminal) {
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
export function removeFromWorkspace(data: string, command: Command, terminal: ITerminal) {
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