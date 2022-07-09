import * as vscode from 'vscode';
import * as fs from 'fs';
import { registerCommands } from './commands';
import { registerFileWatchers } from './fileWatchers';
import { Terminal } from './terminal';
import { createTempDir, deleteTempFiles } from './folderUtils';
import { UserConfig } from './config';
import { Config, ITerminal } from './types';


/* Idea:
                     Exec command in vscode
       ┌───────────────────────────────────────────┐
       │                                           │
       │                                           │
       │              ┌───────────┐         ┌──────┴─────┐
       │              │           │Register │    File    │
       │              │ Extension ├─────────►            │
       │              │           │         │  Watchers  │
       │              └─────┬─────┘         └──────▲─────┘
       │                    │                      │
 ┌─────▼─────┐              │                      │Output selection
 │           │              │Register              │to a file
 │  vscode   │              │                      │
 │           │              │                      │
 ├───────────┤        ┌─────▼─────┐         ┌──────┴─────┐
 │           │        │           │Creates  │            │
 │   User    ├────────►  Command  ├─────────►  Terminal  │
 │           │Invokes │           │         │            │
 └───────────┘        └───────────┘         └────────────┘
*/
let terminal: ITerminal;
let commandsWatchers: fs.FSWatcher[];
let commandRegistration: vscode.Disposable;
let config: Config;

export function activate(context: vscode.ExtensionContext) {
      init();
}

function init() {
      createTempDir();

	config = new UserConfig();
	const configEvent = vscode.workspace.onDidChangeConfiguration(e => {
            configEvent.dispose();
		config.refreshUserSettings();
            clear();
            init();
            initCommands();
	});

	terminal = Terminal.getInstance();
      initCommands();
}

function initCommands() {
      commandsWatchers = registerFileWatchers(config.commands, config, terminal);
      commandRegistration = registerCommands(config.commands, config, terminal);
}

function clear() {
      deleteTempFiles(config.commands);
      terminal.dispose();
      commandsWatchers.forEach(watcher => watcher.close());
      commandRegistration.dispose();
}

export function deactivate() {
      clear();
}