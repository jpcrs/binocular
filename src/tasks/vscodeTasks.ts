import * as vscode from 'vscode';

export class VscodeTasks {
    public moveToEditor : vscode.Task;
    public moveToPanel : vscode.Task;
    public closePanel : vscode.Task;
    public focusTerminal : vscode.Task;
    public focusPanel : vscode.Task;

    constructor()
    {
        this.moveToEditor = this.initVSCodeTask('moveToEditor', 'workbench.action.terminal.moveToEditor');
        this.moveToPanel = this.initVSCodeTask('moveToPanel', 'workbench.action.terminal.moveToPanel');
        this.closePanel = this.initVSCodeTask('closePanel', 'workbench.action.closePanel');
        this.focusTerminal = this.initVSCodeTask('focusTerminal', 'workbench.action.terminal.focus');
        this.focusPanel = this.initVSCodeTask('focusPanel', 'workbench.action.focusPanel');
    }

    private initVSCodeTask(name: string, command: string): vscode.Task {
        let task = new vscode.Task(
            { type: 'shell' },
            vscode.TaskScope.Global,
            `${name}`,
            'binocular',
            new vscode.ShellExecution(`\$\{command:${command}\}`)
        );
        return task;
    }
}