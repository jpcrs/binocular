import * as vscode from 'vscode';
import * as os from 'os';
import { addFolderToWorkspace, changeToWorkspace, openFile, openFileAndJumpToLine, removeFromWorkspace } from './fileWatchers';
import { getConfiguredFolders, getTempFile, getWorkspaceFolders, getWorkspaceFoldersWithLineBreak } from './folderUtils';
import { Config } from './config';
import { EXTENSION_NAME } from './const';
import { ITerminal } from './terminal';

/** A Command is a structure containing all the information necessary to execute the CLI command and pick a handler to interact with VSCode */
export interface Command {
    /** File name that will receive the output from the CLI command. It's also used to register all the filewatchers. */
    fileName: string,

    /** Handler method that will be executed by the fileWatcher whenever the @fileName has an update */
    handler: (data: string, terminal: ITerminal) => void,

    /** CLI command that will be executed in the terminal. It can be overwritten by the user */
    configCommand: (cfg: Config) => string,

    /** Parser method to override some parts of the command in the runtime, like the folders that will be used in the command. */
    parseCommandWithParameters: (cmd: string, cfg: Config) => string,
}

/**
 * All the commands that will be registered by the extension. The key is the name of the command and the value is the command itself.
 */
export const defaultCommands: { [key: string]: Command } = {
    findFilesByNameInCurrentWorkspace: {
        configCommand: (cfg: Config) => cfg.findFilesByNameInCurrentWorkspaceCommand,
        fileName: "openFile",
        handler: openFile,
        parseCommandWithParameters: (cmd: string) => cmd
    },
    findFilesByNameInAllOpenWorkspaces: {
        configCommand: (cfg: Config) => cfg.findFilesByNameInAllWorkspacesCommand,
        fileName: "openFile2",
        handler: openFile,
        parseCommandWithParameters: (cmd: string) => cmd.replaceAll("#", getWorkspaceFolders())
    },
    findFilesByNameInConfiguredFolders: {
        configCommand: (cfg: Config) => cfg.findFilesByNameInConfiguredFoldersCommand,
        fileName: "openFile3",
        handler: openFile,
        parseCommandWithParameters: (cmd: string, cfg: Config) => cmd.replaceAll("#", getConfiguredFolders(cfg))
    },
    findFilesByContentInCurrentWorkspace: {
        configCommand: (cfg: Config) => cfg.findFilesByContentInCurrentWorkspaceCommand,
        fileName: "openFileLine",
        handler: openFileAndJumpToLine,
        parseCommandWithParameters: (cmd: string) => cmd
    },
    findFilesByContentInAllWorkspaces: {
        configCommand: (cfg: Config) => cfg.findFilesByContentInAllWorkspacesCommand,
        fileName: "openFileLine2",
        handler: openFileAndJumpToLine,
        parseCommandWithParameters: (cmd: string) => cmd.replaceAll("#", getWorkspaceFolders())
    },
    findFilesByContentInConfiguredFolders: {
        configCommand: (cfg: Config) => cfg.findFilesByContentInConfiguredFoldersCommand,
        fileName: "openFileLine3",
        handler: openFileAndJumpToLine,
        parseCommandWithParameters: (cmd: string, cfg: Config) => cmd.replaceAll("#", getConfiguredFolders(cfg))
    },
    addFolderToWorkspaceFromConfiguredFolders: {
        configCommand: (cfg: Config) => cfg.addFolderToWorkspaceFromConfiguredFoldersCommand,
        fileName: "addWorkspace",
        handler: addFolderToWorkspace,
        parseCommandWithParameters: (cmd: string, cfg: Config) => cmd.replaceAll("#", getConfiguredFolders(cfg))
    },
    changeToWorkspaceFromConfiguredFolders: {
        configCommand: (cfg: Config) => cfg.changeToWorkspaceFromConfiguredFoldersCommand,
        fileName: "changeWorkspace",
        handler: changeToWorkspace,
        parseCommandWithParameters: (cmd: string, cfg: Config) => cmd.replaceAll("#", getConfiguredFolders(cfg))
    },
    removeFoldersFromWorkspace: {
        configCommand: (cfg: Config) => cfg.removeFoldersFromWorkspaceCommand,
        fileName: "removeWorkspace",
        handler: removeFromWorkspace,
        parseCommandWithParameters: (cmd: string) => cmd.replaceAll("#", getWorkspaceFoldersWithLineBreak())
    },
};

/**
 * Register all the commands that will be used by the extension.
 * @param commands Commands that will be registered by the extension. The key is the identifier in the package.json
 * @param cfg Config object that contains the configuration of the extension.
 * @param terminal Terminal object that will be used to execute the commands.
 */
export function registerCommands(commands: { [key: string]: Command }, cfg: Config, terminal: ITerminal) {
    Object.entries(commands).map(command => {
        vscode.commands.registerCommand(`${EXTENSION_NAME}.${command[0]}`, () => {
            terminal.executeCommand(parseCommand(command[1], cfg));
        });
    });
}

/**
 * Parses the command and replaces the placeholders.
 * @param cmd Command that will be executed by the terminal, with the placeholders
 * @param cfg Config that has to be used to access some configuration values, like the configured folders.
 * @returns Final command that will be executed by the terminal.
 */
function parseCommand(cmd: Command, cfg: Config): string {
    const configuredCommand = cmd.configCommand(cfg).replaceAll("@", getOsPwd());

    if (cfg.externalTerminal) {
        const commandToExecute = `${cmd.parseCommandWithParameters(cfg.externalTerminalCustomCommand.replaceAll("#", configuredCommand), cfg)} > ${getTempFile(cmd.fileName, cfg)}`;
        return commandToExecute;
    }

    const commandToExecute = `${cmd.parseCommandWithParameters(configuredCommand, cfg)} > ${getTempFile(cmd.fileName, cfg)}`;
    return commandToExecute;
}

function getOsPwd(): string {
    switch (os.platform()) {
        case 'win32':
            return '%cd%';
        default:
            return '$(pwd)'
    }
}
