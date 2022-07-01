import * as assert from 'assert';
import * as vscode from 'vscode';
import * as os from 'os';
import { EXTENSION_NAME } from './const';

/**
 * Extension configuration.
 */
export interface Config {
    /**
     * Additional folders to be searched for files.
     */
    readonly additionalFolders: string[] | undefined,

    /**
     * Boolean flag to open the terminal in a new window.
     */
    readonly externalTerminal: boolean

    /**
     * Custom command to be used to open the external terminal. If not set, the default terminal command will be used based on the operating system.
     * @see getDefaultTerminalCommand
     */
    readonly externalTerminalCustomCommand: string,

    /**
     * Command find files by name in the current workspace (Based on the current file in focus).
     * @default rg --files --hidden $(pwd) | fzf --ansi -m --preview 'bat --color=always {}'
     */
    readonly findFilesByNameInCurrentWorkspaceCommand: string,

    /**
     * Command find files by name in all the open workspaces.
     * @default rg --files --hidden $(pwd) # | fzf --ansi -m --preview 'bat --color=always {}'
     * The # is a placeholder for the workspace folders.
     */
    readonly findFilesByNameInAllWorkspacesCommand: string,

    /**
     * Command to find files by name in all the pre-configured folders.
     * @default rg --files --hidden $(pwd) # | fzf --ansi -m --preview 'bat --color=always {}'
     * The # is a placeholder for the configured folders.
     */
    readonly findFilesByNameInConfiguredFoldersCommand: string,

    /**
     * Command to find files by content in all the folders in the current workspace (Based on the current file in focus).
     * @default rg --column --line-number --no-heading --color=always --smart-case . $(pwd) | fzf -m --delimiter : --bind 'change:reload:rg --column --line-number --no-heading --color=always --smart-case {q} $(pwd) || true' --ansi --preview 'bat --color=always {1} --highlight-line {2}'
     */
    readonly findFilesByContentInCurrentWorkspaceCommand: string,

    /**
     * Command to find files by content in all the folders in all the open workspaces.
     * @default rg --column --line-number --no-heading --color=always --smart-case . $(pwd) # | fzf -m --delimiter : --bind 'change:reload:rg --column --line-number --no-heading --color=always --smart-case {q} $(pwd) # || true' --ansi --preview 'bat --color=always {1} --highlight-line {2}'
     * The # is a placeholder for the workspace folders.
     */
    readonly findFilesByContentInAllWorkspacesCommand: string,

    /**
     * Command to find files by content in all the pre-configured folders.
     * @default rg --column --line-number --no-heading --color=always --smart-case . $(pwd) # | fzf -m --delimiter : --bind 'change:reload:rg --column --line-number --no-heading --color=always --smart-case {q} $(pwd) # || true' --ansi --preview 'bat --color=always {1} --highlight-line {2}'
     * The # is a placeholder for the configured folders.
     */
    readonly findFilesByContentInConfiguredFoldersCommand: string,

    /**
     * Command to add new folders to the workspace. It searches for folders that contains a .git directory inside.
     * @default fd .git$ -td -H --absolute-path # | sed 's/\\/.git//g' | fzf -m
     * The # is a placeholder for the configured folders.
     */
    readonly addFolderToWorkspaceFromConfiguredFoldersCommand: string,

    /**
     * Command to search for folders and make it the current main workspace (All the other workspaces will be closed).
     * @default fd .git$ -td -H --absolute-path # | sed 's/\\/.git//g' | fzf
     * The # is a placeholder for the configured folders.
     */
    readonly changeToWorkspaceFromConfiguredFoldersCommand: string,

    /**
     * Command to list all the open workspaces and close the selected ones.
     * @default echo # | fzf -m
     * The # is a placeholder for all the open workspaces.
     */
    readonly removeFoldersFromWorkspaceCommand: string,

    /**
     * Guid of the extension. It's used to identify the temporary files that will be used as output for the terminal commands.
     */
    readonly guid: string,
}

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
    guid: string;

    constructor() {
        this.additionalFolders = this.getCFG<string[]>('general.additionalSearchLocations');
        this.externalTerminal = this.getCFG<boolean>('general.useExternalTerminal');
        this.externalTerminalCustomCommand = this.getCFG<string>('command.externalTerminalCustomCommand') !== '' ? this.getCFG<string>('command.externalTerminalCustomCommand') :  getDefaultTerminalCommand();
        this.findFilesByNameInCurrentWorkspaceCommand = this.getCFG<string>('command.findFilesByNameInCurrentWorkspaceCommand');
        this.findFilesByNameInAllWorkspacesCommand = this.getCFG<string>('command.findFilesByNameInAllWorkspacesCommand');
        this.findFilesByNameInConfiguredFoldersCommand = this.getCFG<string>('command.findFilesByNameInConfiguredFoldersCommand');
        this.findFilesByContentInCurrentWorkspaceCommand = this.getCFG<string>('command.findFilesByContentInCurrentWorkspaceCommand');
        this.findFilesByContentInAllWorkspacesCommand = this.getCFG<string>('command.findFilesByContentInAllWorkspacesCommand');
        this.findFilesByContentInConfiguredFoldersCommand = this.getCFG<string>('command.findFilesByContentInConfiguredFoldersCommand');
        this.addFolderToWorkspaceFromConfiguredFoldersCommand = this.getCFG<string>('command.addFolderToWorkspaceFromConfiguredFoldersCommand');
        this.changeToWorkspaceFromConfiguredFoldersCommand = this.getCFG<string>('command.changeToWorkspaceFromConfiguredFoldersCommand');
        this.removeFoldersFromWorkspaceCommand = this.getCFG<string>('command.removeFoldersFromWorkspaceCommand');
        this.guid = this.generateGuid();
    }

    /**
     * Reload the user config. Ideally invoked in the onDidChangeConfiguration event.
     */
    public updateUserSettings() {
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
    }

    generateGuid(): string {
        return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function(c) {
            var r = Math.random() * 16 | 0, v = c === 'x' ? r : (r & 0x3 | 0x8);
            return v.toString(16);
        });
    }

    getCFG<T>(key: string) {
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
            return 'start cmd /k "# & exit /s"';
        case 'darwin':
            return `osascript -e 'tell app "Terminal" to do script "ls" & activate & do script "#;exit"`;
        default:
            return `gnome-terminal -- sh -c "#"`;
    }
}
