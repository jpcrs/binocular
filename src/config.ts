import * as assert from 'assert';
import * as vscode from 'vscode';
import * as os from 'os';
import { EXTENSION_NAME, ExternalTerminalCommands } from './constants';
import { Config, CustomCommands } from './types';

export class UserConfig implements Config {
    additionalFolders: string[] | undefined;
    externalTerminal: boolean;
    externalTerminalCustomCommand: string;
    findFilesByNameInCurrentWorkspaceCommand: string;
    findFilesByNameInAllWorkspacesCommand: string;
    findFilesByNameInConfiguredFoldersCommand: string;
    findFilesByContentInCurrentWorkspaceCommand: string;
    findFilesByContentInAllWorkspacesCommand: string;
    findFilesByContentInConfiguredFoldersCommand: string;
    addFolderToWorkspaceFromConfiguredFoldersCommand: string;
    changeToWorkspaceFromConfiguredFoldersCommand: string;
    removeFoldersFromWorkspaceCommand: string;
    customCommands: CustomCommands[];
    guid: string;

    constructor() {
        this.additionalFolders = this.getCFG<string[]>('general.additionalSearchLocations');
        this.externalTerminal = this.getCFG<boolean>('general.useExternalTerminal');
        this.externalTerminalCustomCommand = this.getCFG<string>('command.externalTerminalCustomCommand') !== '' ? this.getCFG<string>('command.externalTerminalCustomCommand') : getDefaultTerminalCommand();
        this.findFilesByNameInCurrentWorkspaceCommand = this.getCFG<string>('command.findFilesByNameInCurrentWorkspaceCommand');
        this.findFilesByNameInAllWorkspacesCommand = this.getCFG<string>('command.findFilesByNameInAllWorkspacesCommand');
        this.findFilesByNameInConfiguredFoldersCommand = this.getCFG<string>('command.findFilesByNameInConfiguredFoldersCommand');
        this.findFilesByContentInCurrentWorkspaceCommand = this.getCFG<string>('command.findFilesByContentInCurrentWorkspaceCommand');
        this.findFilesByContentInAllWorkspacesCommand = this.getCFG<string>('command.findFilesByContentInAllWorkspacesCommand');
        this.findFilesByContentInConfiguredFoldersCommand = this.getCFG<string>('command.findFilesByContentInConfiguredFoldersCommand');
        this.addFolderToWorkspaceFromConfiguredFoldersCommand = this.getCFG<string>('command.addFolderToWorkspaceFromConfiguredFoldersCommand');
        this.changeToWorkspaceFromConfiguredFoldersCommand = this.getCFG<string>('command.changeToWorkspaceFromConfiguredFoldersCommand');
        this.removeFoldersFromWorkspaceCommand = this.getCFG<string>('command.removeFoldersFromWorkspaceCommand');
        this.customCommands = this.getCFG<CustomCommands[]>('command.customCommands');
        this.guid = this.generateGuid();
    }

    /**
     * Reload the user config. Ideally invoked in the onDidChangeConfiguration event.
     */
    public refreshUserSettings() {
        this.additionalFolders = this.getCFG<string[]>('general.additionalSearchLocations');
        this.externalTerminal = this.getCFG<boolean>('general.useExternalTerminal');
        this.externalTerminalCustomCommand = this.getCFG<string>('command.externalTerminalCustomCommand');
        this.findFilesByNameInCurrentWorkspaceCommand = this.getCFG<string>('command.findFilesByNameInCurrentWorkspaceCommand');
        this.findFilesByNameInAllWorkspacesCommand = this.getCFG<string>('command.findFilesByNameInAllWorkspacesCommand');
        this.findFilesByNameInConfiguredFoldersCommand = this.getCFG<string>('command.findFilesByNameInConfiguredFoldersCommand');
        this.findFilesByContentInCurrentWorkspaceCommand = this.getCFG<string>('command.findFilesByContentInCurrentWorkspaceCommand');
        this.findFilesByContentInAllWorkspacesCommand = this.getCFG<string>('command.findFilesByContentInAllWorkspacesCommand');
        this.findFilesByContentInConfiguredFoldersCommand = this.getCFG<string>('command.findFilesByContentInConfiguredFoldersCommand');
        this.addFolderToWorkspaceFromConfiguredFoldersCommand = this.getCFG<string>('command.addFolderToWorkspaceFromConfiguredFoldersCommand');
        this.changeToWorkspaceFromConfiguredFoldersCommand = this.getCFG<string>('command.changeToWorkspaceFromConfiguredFoldersCommand');
        this.removeFoldersFromWorkspaceCommand = this.getCFG<string>('command.removeFoldersFromWorkspaceCommand');
        this.customCommands = this.getCFG<CustomCommands[]>('command.customCommands');
    }

    generateGuid(): string {
        return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function(c) {
            var r = Math.random() * 16 | 0, v = c === 'x' ? r : (r & 0x3 | 0x8);
            return v.toString(16);
        });
    }

    getCFG<T>(key: string): T {
        const userCfg = vscode.workspace.getConfiguration();
        const ret = userCfg.get<T>(`${EXTENSION_NAME}.${key}`);
        if(ret === undefined)
        {
            console.log("Config key not found: " + key);
        }
        assert(ret !== undefined);
        return ret;
    }
}

/**
 * Method to decide which command to use depending on the OS.
 * @returns {string} Command to execute the cmd in a new terminal window and close it after. The # is a placeholder for the cmd.
 * TODO: Currently hardcoded to use gnome-terminal and cmd.exe, maybe we can be smarter and pick the default one using x-terminal-emulator and etc?
 */
function getDefaultTerminalCommand(): string {
    switch (os.platform()) {
        case 'win32':
            return ExternalTerminalCommands.windows;
        case 'darwin':
            return ExternalTerminalCommands.macOs;
        default:
            return ExternalTerminalCommands.linux;
    }
}
