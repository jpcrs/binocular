import * as path from 'path';
import * as os from 'os';
import * as fs from 'fs' ;
import { EXTENSION_NAME } from './constants';
import { Command, Config } from './types';

/**
 * 
 * @param fileName Name of the file to be created.
 * @param cfg Config, used to get the guid for this vscode instance.
 * @returns The temporary folder for new files being watched by filewatcher.
 */
export function getTempFile(fileName: string, cfg: Config): string {
    return `${os.tmpdir()}${path.sep}${EXTENSION_NAME}${path.sep}${fileName}`;
}

export function deleteTempFiles(commands: Command[]) {
    commands.forEach(command => {
        const file = `${os.tmpdir()}${path.sep}${EXTENSION_NAME}${path.sep}${command.outputFile}`;
        if (!fs.existsSync(file)) {
            return;
        }
        fs.rmSync(file);
    });
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