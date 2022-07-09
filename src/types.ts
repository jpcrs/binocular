
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
     * All the commands registered.
     */
    readonly commands: Command[],

    refreshUserSettings(): void,
}

export interface Command {
    shellCommand: string;
    commandIdentifier: string;
    script: string;
    outputFile: string;
    handler: (data: string, command: Command, terminal: ITerminal) => void;
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