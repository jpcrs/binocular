import * as vscode from 'vscode';
import { EXTENSION_NAME } from './const';

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

export class Terminal implements ITerminal {
    private static instance: Terminal;
    private vscodeTerminal: vscode.Terminal;

    private constructor() {
        this.vscodeTerminal = vscode.window.createTerminal({name: EXTENSION_NAME, location: 2 });
        this.vscodeTerminal.dispose();
    }

    public static getInstance(): ITerminal {
        if (!Terminal.instance) {
            Terminal.instance = new Terminal();
        }
        return Terminal.instance;
    }

    public executeCommand(cmd: string): void {
        this.listenTerminalFocusEvent();
        this.show();

        this.vscodeTerminal.sendText(cmd);
    }

    /**
     * TODO: Check why this is not setting the exitStatus.
     */
    public dispose() {
        this.vscodeTerminal.dispose();
    }
    
    private listenTerminalFocusEvent() {
        const disposable = vscode.window.onDidChangeTerminalState(x => {
            if (x.name === EXTENSION_NAME){
                x.show();
            }
            disposable.dispose();
        });
    }

    /**
     * TODO: After we fix the exitStatus, we can stop using the try catch to discover if we have to create a new terminal or not.
     */
    private show() {
        try {
            this.vscodeTerminal.show();
        } catch (e) {
            this.vscodeTerminal = vscode.window.createTerminal({name: EXTENSION_NAME, location: 2 });
            this.vscodeTerminal.show();
        }
    }
}