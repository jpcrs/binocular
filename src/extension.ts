import * as vscode from 'vscode';
import * as fs from 'fs';
import { defaultCommands, parseCustomCommandToCommand, registerCommands, registerCustomCommands } from './commands';
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
let customCommandsWatchers: fs.FSWatcher[];
let defaultCommandsWatchers: fs.FSWatcher[];
let defaultCommandRegistrations: vscode.Disposable[];
let customCommandRegistration: vscode.Disposable;
let config: Config;

export function activate(context: vscode.ExtensionContext) {
      init();
      initCustomCommands();
      initDefaultCommands();
}

function init() {
      createTempDir();

	config = new UserConfig();
	vscode.workspace.onDidChangeConfiguration(e => {
		config.refreshUserSettings();
            clear();
            init();
            initCustomCommands();
            initDefaultCommands();
	});

	terminal = Terminal.getInstance();
}

function initDefaultCommands() {
	defaultCommandsWatchers = registerFileWatchers(defaultCommands, config, terminal);
	defaultCommandRegistrations = registerCommands(defaultCommands, config, terminal);
}

function initCustomCommands() {
      const customCommands = parseCustomCommandToCommand(config.customCommands);
      customCommandsWatchers = registerFileWatchers(customCommands, config, terminal);
      customCommandRegistration = registerCustomCommands(customCommands, config, terminal);
}

function clear() {
      deleteTempFiles(config);
      terminal.dispose();
      customCommandsWatchers.forEach(watcher => watcher.close());
      defaultCommandsWatchers.forEach(watcher => watcher.close());
      defaultCommandRegistrations.forEach(command => command.dispose());
      customCommandRegistration.dispose();
}

export function deactivate() {
      clear();
}