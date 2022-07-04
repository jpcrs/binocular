
/** A Command is a structure containing all the information necessary to execute the CLI command and pick a handler to interact with VSCode */
export interface Command {
    /** Identifier for the command, used for the command registration. */
    commandIdentifier: string,

    /** File name that will receive the output from the CLI command. It's also used to register all the filewatchers. */
    outputFile: string,

    /** Handler method that will be executed by the fileWatcher whenever the @fileName has an update. */
    handler: (data: string, command: Command, terminal: ITerminal) => void,

    /** Shell command that will be executed in the terminal. It can be overwritten by the user. */
    shellCommand: (cfg: Config) => string,

    /** Script path */
    scriptPath?: string,
}


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

    readonly customCommands: CustomCommands[],

    /**
     * Guid of the extension. It's used to identify the temporary files that will be used as output for the terminal commands.
     */
    readonly guid: string,
}

export interface CustomCommands {
    commandIdentifier: string;
    outputFile: string;
    shellCommand: string;
    scriptPath: string;
}

/**
 * VSCode terminal wrapper used to execute our commands.
 */
export interface ITerminal {
    /**
     * Receives a command, created a terminal if necessary and executes it.
     * @param cmd Command to be executed.
     */
    executeCommand(cmd: string): void;

    /**
     * Dispose the terminal
     */
    dispose(): void;
}