import * as vscode from 'vscode';
import * as path from 'path';
import * as os from 'os';
import * as fs from 'fs' ;
import { EXTENSION_NAME } from './constants';
import { Config } from './types';

/**
 * 
 * @param fileName Name of the file to be created.
 * @param cfg Config, used to get the guid for this vscode instance.
 * @returns The temporary folder for new files being watched by filewatcher.
 */
export function getTempFile(fileName: string, cfg: Config): string {
    return `${os.tmpdir()}${path.sep}${EXTENSION_NAME}${path.sep}${fileName}-${cfg.guid}`;
}

export function deleteTempFiles(cfg: Config): string[] {
    var files = fs.readdirSync(`${os.tmpdir()}${path.sep}${EXTENSION_NAME}`).filter(fn => fn.endsWith(`-${cfg.guid}`));
    files.forEach(file => {
        fs.rmSync(`${os.tmpdir()}${path.sep}${EXTENSION_NAME}${path.sep}${file}`);
    });
    return files;
}

/**
 * Creates the temporary directory used by the plugin.
 */
export function createTempDir(): void {
    const tempDir = `${os.tmpdir()}${path.sep}${EXTENSION_NAME}`;
    if (!fs.existsSync(tempDir)) {
        fs.mkdirSync(tempDir);
    }
}   