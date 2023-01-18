import * as vscode from 'vscode';
import * as os from 'os';

export enum VSCodeVersion {
    Code,
    Insiders
}

export function getVscodeVersion(): VSCodeVersion {
    if (vscode.version.startsWith("i"))
        return VSCodeVersion.Insiders;

    return VSCodeVersion.Code;
}

export function getFileHistoryPath(): string {
    let historyPath;
    let version = getVscodeVersion();
    switch (os.platform()) {
        case 'win32':
            historyPath = version == VSCodeVersion.Code ? `${process.env.APPDATA}\\Code\\User\\history` : `${process.env.APPDATA}\\Code-Insiders\\User\\history`;
            break;
        case 'darwin':
            historyPath = version == VSCodeVersion.Code ? `${os.homedir()}/Library/Application Support/Code/User/History` : `${os.homedir()}/Library/Application Support/Code - Insiders/User/History`;
            break;
        case 'linux':
            historyPath = version == VSCodeVersion.Code ? `${os.homedir()}/.config/Code/User/history` : `${os.homedir()}/.config/Code - Insiders/User/history`;
            break;
        default:
            throw new Error(`Unsupported platform: ${os.platform()}`);
    }
    return historyPath;
}