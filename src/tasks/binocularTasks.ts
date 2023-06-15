import * as vscode from 'vscode';
import { UserConfig } from '../config';
import { writeFile } from '../fileWriter';
import { getFileHistoryPath, getVscodeVersion, VSCodeVersion } from '../helpers';

export class BinocularTasks {
    private opt: string;
    private additionalFolders: string;

    constructor(userConfig: UserConfig)
    {
        this.additionalFolders = userConfig.additionalFolders?.map(x => `-p ${x}`).join(" ") ?? "";
        this.opt = getVscodeVersion() == VSCodeVersion.Code ? "-c" : "-i";
    }

    public searchFile = (): vscode.Task =>
        this.initBinocularTask(`binocular-cli ${this.opt} -f ${vscode.workspace.workspaceFolders?.map(x => `-p ${x.uri.fsPath}`).join(" ") ?? ""}`);

    public searchFileContent = (): vscode.Task =>
        this.initBinocularTask(`binocular-cli ${this.opt} ${vscode.workspace.workspaceFolders?.map(x => `-p ${x.uri.fsPath}`).join(" ") ?? ""}`);

    public searchDirectory = (): vscode.Task =>
        this.initBinocularTask(`binocular-cli ${this.opt} -d ${vscode.workspace.workspaceFolders?.map(x => `-p ${x.uri.fsPath}`).join(" ") ?? ""}`);

    public searchFileConfiguredFolders = (): vscode.Task =>
        this.initBinocularTask(`binocular-cli ${this.opt} -f ${this.additionalFolders}`);

    public searchFileContentConfiguredFolders = (): vscode.Task =>
        this.initBinocularTask(`binocular-cli ${this.opt} ${this.additionalFolders}`);

    public searchDirectoryConfiguredFolders = (): vscode.Task =>
        this.initBinocularTask(`binocular-cli ${this.opt} -d ${this.additionalFolders}`);

    public git(userConfig: UserConfig): vscode.Task {
        let folderLocation = getFileHistoryPath();
        let activeTextEditor = vscode.window.activeTextEditor?.document.uri.fsPath!;
        const config = vscode.workspace.getConfiguration();
        const historyPath = config.get('history.location');
        return this.initBinocularTask(`binocular-cli ${this.opt} --git ${this.additionalFolders}`);
    }

    public history(): vscode.Task {
        let folderLocation = getFileHistoryPath();
        let activeTextEditor = vscode.window.activeTextEditor?.document.uri.fsPath!;
        const config = vscode.workspace.getConfiguration();
        const historyPath = config.get('history.location');
        console.log(historyPath);
        return this.initBinocularTask(`binocular-cli ${this.opt} history -p '${folderLocation}' -f '${activeTextEditor}'`);
    }

    public diagnostic(severity: vscode.DiagnosticSeverity): vscode.Task {
        let diagnostics = vscode.languages.getDiagnostics().filter(x => x[1].some(z => z.severity == severity));
		let fileContent = '';
		for (let i = 0; i < diagnostics.length; i++) {
			let diagnostic = diagnostics[i];
            let errors = diagnostic[1].filter(x => x.severity == severity);
            for (let j = 0; j < errors.length; j++) {
                let error = errors[j];
                fileContent += `${diagnostic[0].fsPath}:${error.range.start.line+1}:${error.range.start.character+1}:${error.range.end.character+1}${error.message}\n`;
            }
		}
		let file = writeFile("diagnostics", fileContent);
        return this.initBinocularTask(`binocular-cli ${this.opt} read-file -f ${file}`);
    }

    public breakpoints(): vscode.Task {
		let fileContent = '';
		let breakpoints = vscode.debug.breakpoints;
		for(let i = 0; i < breakpoints.length; i++)
		{
			let bp = breakpoints[i];
			if (bp instanceof vscode.SourceBreakpoint) {
				let composedString = `${bp.location.uri.fsPath}:${bp.location.range.start.line}:${bp.location.range.start.character}:${bp.condition != undefined ? bp.condition : ""}\n`
				fileContent += composedString;
			}
		}
		let file = writeFile("breakpoints", fileContent);

        return this.initBinocularTask(`binocular-cli -c read-file -f ${file}`);
    }

    private initBinocularTask(command: string): vscode.Task {
        let task = new vscode.Task(
            { type: 'shell' },
            vscode.TaskScope.Workspace,
            'binocular',
            'binocular',
            new vscode.ShellExecution(`${command}`),
            '$binocular',
        )
        task.presentationOptions = {
            reveal: vscode.TaskRevealKind.Always,
            focus: true,
            panel: vscode.TaskPanelKind.Dedicated,
            showReuseMessage: false,
            clear: true,
            close: true
        }
        task.isBackground = true;
        return task;
    }

    public customTask(command: string): vscode.Task {
        let task = new vscode.Task(
            { type: 'shell' },
            vscode.TaskScope.Workspace,
            'binocular',
            'binocular',
            new vscode.ShellExecution(`${command}`),
            '$binocular'
        )
        task.presentationOptions = {
            reveal: vscode.TaskRevealKind.Always,
            focus: true,
            panel: vscode.TaskPanelKind.Dedicated,
            showReuseMessage: false,
            clear: true
        }
        task.isBackground = true;
        return task;
    }
}
