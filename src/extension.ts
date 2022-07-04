import * as vscode from 'vscode';
import * as fs from 'fs';
import { defaultCommands, registerCommands, registerCustomCommands } from './commands';
import { registerFileWatchers } from './fileWatchers';
import { Terminal } from './terminal';
import { createTempDir } from './folderUtils';
import { UserConfig } from './config';


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
export function activate(context: vscode.ExtensionContext) {
      createTempDir();

	var config = new UserConfig();
	vscode.workspace.onDidChangeConfiguration(e => {
		config.updateUserSettings();
	});

	const terminal = Terminal.getInstance();

      const customCommands = registerCustomCommands(config, terminal);
      registerFileWatchers(customCommands, config, terminal);
	registerCommands(defaultCommands, config, terminal);
	registerFileWatchers(defaultCommands, config, terminal);
}

export function deactivate() {
      //TODO: Dispose terminal in case it's still open.
      //TODO: Delete all the tmp files related to this session.
      //TODO: Maybe we have to stop all the file watchers?
}