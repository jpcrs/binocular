import * as vscode from 'vscode';
import { addFolderToWorkspace, changeToWorkspace, executeCustomCommand, openFile, openFileAndJumpToLine, removeFromWorkspace } from './fileWatchers';
import { EXTENSION_NAME } from './constants';
import { Command, Config, CustomCommands, ITerminal } from './types';
import { parseCommand } from './commandParsers';

/**
 * All the commands that will be registered by the extension. The key is the name of the command and the value is the command itself.
 */
export const defaultCommands: Command[] = [
    {
        commandIdentifier: 'findFilesByNameInCurrentWorkspace',
        shellCommand: (cfg: Config) => cfg.findFilesByNameInCurrentWorkspaceCommand,
        outputFile: "openFile",
        handler: openFile,
    },
    {
        commandIdentifier: 'findFilesByNameInAllOpenWorkspaces',
        shellCommand: (cfg: Config) => cfg.findFilesByNameInAllWorkspacesCommand,
        outputFile: "openFile2",
        handler: openFile,
    },
    {
        commandIdentifier: 'findFilesByNameInConfiguredFolders',
        shellCommand: (cfg: Config) => cfg.findFilesByNameInConfiguredFoldersCommand,
        outputFile: "openFile3",
        handler: openFile,
    },
    {
        commandIdentifier: 'findFilesByContentInCurrentWorkspace',
        shellCommand: (cfg: Config) => cfg.findFilesByContentInCurrentWorkspaceCommand,
        outputFile: "openFileLine",
        handler: openFileAndJumpToLine,
    },
    {
        commandIdentifier: 'findFilesByContentInAllWorkspaces',
        shellCommand: (cfg: Config) => cfg.findFilesByContentInAllWorkspacesCommand,
        outputFile: "openFileLine2",
        handler: openFileAndJumpToLine,
    },
    {
        commandIdentifier: 'findFilesByContentInConfiguredFolders',
        shellCommand: (cfg: Config) => cfg.findFilesByContentInConfiguredFoldersCommand,
        outputFile: "openFileLine3",
        handler: openFileAndJumpToLine,
    },
    {
        commandIdentifier: 'addFolderToWorkspaceFromConfiguredFolders',
        shellCommand: (cfg: Config) => cfg.addFolderToWorkspaceFromConfiguredFoldersCommand,
        outputFile: "addWorkspace",
        handler: addFolderToWorkspace,
    },
    {
        commandIdentifier: 'changeToWorkspaceFromConfiguredFolders',
        shellCommand: (cfg: Config) => cfg.changeToWorkspaceFromConfiguredFoldersCommand,
        outputFile: "changeWorkspace",
        handler: changeToWorkspace,
    },
    {
        commandIdentifier: 'removeFoldersFromWorkspace',
        shellCommand: (cfg: Config) => cfg.removeFoldersFromWorkspaceCommand,
        outputFile: "removeWorkspace",
        handler: removeFromWorkspace,
    },
];

/**
 * Register all the commands that will be used by the extension.
 * @param commands Commands that will be registered by the extension. The key is the identifier in the package.json
 * @param cfg Config object that contains the configuration of the extension.
 * @param terminal Terminal object that will be used to execute the commands.
 */
export function registerCommands(commands: Command[], cfg: Config, terminal: ITerminal) : vscode.Disposable[] {
    return commands.map(command => 
        vscode.commands.registerCommand(`${EXTENSION_NAME}.${command.commandIdentifier}`, () => {
            terminal.executeCommand(parseCommand(command, cfg));
        })
    );
}


/**
 * Register all the custom commands provided by the user.
 * @param cfg Config object that contains the configuration of the extension.
 * @param terminal Terminal object that will be used to execute the commands.
 */
export function registerCustomCommands(commands: Command[], cfg: Config, terminal: ITerminal): vscode.Disposable {
    return vscode.commands.registerCommand(`${EXTENSION_NAME}.customCommands`, async (commandIdentifier: string) => {
        if (!commandIdentifier) {
            commandIdentifier = await vscode.window.showQuickPick(cfg.customCommands.map(x => x.commandIdentifier)) ?? commandIdentifier;
        }
        var command = commands.find(x => x.commandIdentifier === commandIdentifier);
        if (command) {
            terminal.executeCommand(parseCommand(command, cfg));
        }
    });
}

export function parseCustomCommandToCommand(customCommands: CustomCommands[]): Command[] {
    return customCommands.map(customCommand => ({ 
        commandIdentifier: customCommand.commandIdentifier, 
        outputFile: customCommand.outputFile, 
        shellCommand: (cfg: Config) => customCommand.shellCommand, 
        handler: executeCustomCommand,
        scriptPath: customCommand.scriptPath
    }));
}
