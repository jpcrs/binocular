import * as vscode from 'vscode';
import { UserConfig } from './config';
import { EXTENSION_NAME } from './constants';
import { BinocularTasks } from './tasks/binocularTasks';
import { VscodeTasks } from './tasks/vscodeTasks';

let onDidStartTask: vscode.Disposable;
let onDidEndTask: vscode.Disposable;
export async function activate(context: vscode.ExtensionContext) {
	let cfg = new UserConfig();
	const configEvent = vscode.workspace.onDidChangeConfiguration(e => {
			cfg.refreshUserSettings();
            clear(context);
            init(context, cfg);
	});

	init(context, cfg);
}

function clear(context: vscode.ExtensionContext) {
	onDidEndTask.dispose();
	onDidStartTask.dispose();
	context.subscriptions.forEach(x => x.dispose());
}

function init(context: vscode.ExtensionContext, cfg: UserConfig) {
	let binocularTasks = new BinocularTasks(cfg);
	let vscodeTasks = new VscodeTasks();

	onDidStartTask?.dispose();
	onDidEndTask?.dispose();

	onDidStartTask = vscode.tasks.onDidStartTask((e) => {
		if (e.execution.task.name === 'binocular') {
			vscode.tasks.executeTask(vscodeTasks.moveToEditor);
			vscode.tasks.executeTask(vscodeTasks.closePanel);
		}
	});
	onDidEndTask = vscode.tasks.onDidEndTask((e) => {
		if (e.execution.task.name === 'binocular') {
			if (cfg.keepTerminalOpen)
			{
				vscode.tasks.executeTask(vscodeTasks.focusTerminal);
				vscode.tasks.executeTask(vscodeTasks.focusTerminal);
			}
		}
	});

	context.subscriptions.push(
		vscode.commands.registerCommand('binocular.searchFile', async () => {
			vscode.tasks.executeTask(binocularTasks.searchFile());
		})
	);
	context.subscriptions.push(
		vscode.commands.registerCommand('binocular.searchFileContent', async () => {
			vscode.tasks.executeTask(binocularTasks.searchFileContent());
		})
	);
	context.subscriptions.push(
		vscode.commands.registerCommand('binocular.searchDirectory', async () => {
			vscode.tasks.executeTask(binocularTasks.searchDirectory());
		})
	);
	context.subscriptions.push(
		vscode.commands.registerCommand('binocular.searchFileConfiguredFolders', async () => {
			vscode.tasks.executeTask(binocularTasks.searchFileConfiguredFolders());
		})
	);
	context.subscriptions.push(
		vscode.commands.registerCommand('binocular.searchFileContentConfiguredFolders', async () => {
			vscode.tasks.executeTask(binocularTasks.searchFileContentConfiguredFolders());
		})
	);
	context.subscriptions.push(
		vscode.commands.registerCommand('binocular.searchDirectoryConfiguredFolders', async () => {
			vscode.tasks.executeTask(binocularTasks.searchDirectoryConfiguredFolders());
		})
	);
	context.subscriptions.push(
		vscode.commands.registerCommand('binocular.searchFileHistory', async () => {
			vscode.tasks.executeTask(binocularTasks.history());
		})
	);
	context.subscriptions.push(
		vscode.commands.registerCommand('binocular.searchGitFoldersConfiguredFolders', async () => {
			vscode.tasks.executeTask(binocularTasks.git(cfg));
		})
	);
	context.subscriptions.push(
		vscode.commands.registerCommand('binocular.searchErrors', async () => {
			vscode.tasks.executeTask(binocularTasks.diagnostic(vscode.DiagnosticSeverity.Error));
		})
	);
	context.subscriptions.push(
		vscode.commands.registerCommand('binocular.searchWarnings', async () => {
			vscode.tasks.executeTask(binocularTasks.diagnostic(vscode.DiagnosticSeverity.Warning));
		})
	);
	context.subscriptions.push(
		vscode.commands.registerCommand('binocular.searchHints', async () => {
			vscode.tasks.executeTask(binocularTasks.diagnostic(vscode.DiagnosticSeverity.Hint));
		})
	);
	context.subscriptions.push(
		vscode.commands.registerCommand(`${EXTENSION_NAME}.customCommands`, async (commandIdentifier: string) => {
			if (!commandIdentifier) {
				commandIdentifier = await vscode.window.showQuickPick(cfg.commands.map(x => x.commandIdentifier)) ?? commandIdentifier;
			}
			var command = cfg.commands.find(x => x.commandIdentifier === commandIdentifier);
			if (command) {
				vscode.tasks.executeTask(binocularTasks.customTask(command.shellCommand));
			}
		})
	);
}

export function deactivate() { 
	onDidStartTask.dispose();
	onDidEndTask.dispose();
}