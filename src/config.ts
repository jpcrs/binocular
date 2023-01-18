import * as assert from 'assert';
import * as vscode from 'vscode';
import { EXTENSION_NAME } from './constants';

export interface Command {
    shellCommand: string;
    commandIdentifier: string;
}

export class UserConfig {
    additionalFolders: string[] | undefined;
    keepTerminalOpen: boolean;
    commands: Command[];

    constructor() {
        this.additionalFolders = this.getCFG<string[]>('general.additionalSearchLocations');
        this.keepTerminalOpen = this.getCFG<boolean>('general.keepTerminalPanelOpenAfterExecution');
        this.commands = this.getCFG<Command[]>('command.commands');
    }

    public refreshUserSettings() {
        this.additionalFolders = this.getCFG<string[]>('general.additionalSearchLocations');
        this.commands = this.getCFG<Command[]>('command.commands');
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