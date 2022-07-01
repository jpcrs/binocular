import * as vscode from 'vscode';
import { defaultCommands, registerCommands } from './commands';
import { FileHandler, registerFileWatchers } from './fileWatchers';
import { UserConfig } from './config';
import { Terminal } from './terminal';


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
	let fileHandlers = Object.entries(defaultCommands).map(x => <FileHandler>{fileName: x[1].fileName, handler: x[1].handler});

	var config = new UserConfig();
	vscode.workspace.onDidChangeConfiguration(e => {
		config.updateUserSettings();
	});

	const terminal = Terminal.getInstance();

	registerCommands(defaultCommands, config, terminal);
	registerFileWatchers(fileHandlers, config, terminal);
}

export function deactivate() {
      //TODO: Dispose terminal in case it's still open.
      //TODO: Delete all the tmp files related to this session.
      //TODO: Maybe we have to stop all the file watchers?
}