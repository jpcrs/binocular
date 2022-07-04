import * as vscode from 'vscode';
import * as path from 'path';
import { tmpdir } from 'os';
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
    return `${tmpdir()}${path.sep}${EXTENSION_NAME}${path.sep}${fileName}-${cfg.guid}`;
}

/**
 * Creates the temporary directory used by the plugin.
 */
export function createTempDir(): void {
    const tempDir = `${tmpdir()}${path.sep}${EXTENSION_NAME}`;
    if (!fs.existsSync(tempDir)) {
        fs.mkdirSync(tempDir);
    }
}   