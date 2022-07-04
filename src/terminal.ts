import * as vscode from 'vscode';
import { EXTENSION_NAME } from './constants';
import { ITerminal } from './types';

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

    public dispose() {
        this.vscodeTerminal.dispose();
    }
    
    private listenTerminalFocusEvent() {
        const disposable = vscode.window.onDidChangeTerminalState(terminal => {
            if (terminal.name === EXTENSION_NAME){
                terminal.show();
            }
            disposable.dispose();
        });
    }

    private show() {
        if (this.vscodeTerminal.exitStatus)
            this.vscodeTerminal = vscode.window.createTerminal({name: EXTENSION_NAME, location: 2 });
        
        try {
            this.vscodeTerminal.show();
        }
        catch(e) { // It'll throw in case it's disposed. Apparently vscode doesn't have an API to check if it's disposed or not.
            this.vscodeTerminal = vscode.window.createTerminal({name: EXTENSION_NAME, location: 2 });
            this.vscodeTerminal.show();
        }
    }
}