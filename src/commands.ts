import * as vscode from 'vscode';
import { addFolderToWorkspace, changeToWorkspace, executeCustomCommand, openFile, openFileAndJumpToLine, removeFromWorkspace } from './fileWatchers';
import { EXTENSION_NAME } from './constants';
import { Command, Config, ITerminal } from './types';
import { parseCommand } from './commandParsers';

const handlers: {[key: string]: (data: string, command: Command, terminal: ITerminal) => void} = {
    'openFile': openFile,
    'openFileAndJumpToLine': openFileAndJumpToLine,
    'addFolderToWorkspace': addFolderToWorkspace,
    'changeToWorkspace': changeToWorkspace,
    'removeFromWorkspace': removeFromWorkspace
};

export function setCommandHandler(commands: Command[]) {
    commands.forEach(command => {
        command.handler = handlers[command.script] ?? executeCustomCommand;
    });
}

/**
 * Register all the custom commands provided by the user.
 * @param cfg Config object that contains the configuration of the extension.
 * @param terminal Terminal object that will be used to execute the commands.
 */
export function registerCommands(commands: Command[], cfg: Config, terminal: ITerminal): vscode.Disposable {
    return vscode.commands.registerCommand(`${EXTENSION_NAME}.executeCommand`, async (commandIdentifier: string) => {
        if (!commandIdentifier) {
            commandIdentifier = await vscode.window.showQuickPick(cfg.commands.map(x => x.commandIdentifier)) ?? commandIdentifier;
        }
        var command = commands.find(x => x.commandIdentifier === commandIdentifier);
        if (command) {
            terminal.executeCommand(parseCommand(command, cfg));
        }
    });
}