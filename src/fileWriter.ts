import * as fs from "fs";
import path = require('path');
import os = require('os');

export function writeFile(fileName: string, content: string): string {
    const tmpDir = os.tmpdir();
    const filePath = path.join(tmpDir, `binocular-${fileName}`);
    fs.writeFile(filePath, content, (err) => {
        if (err) {
            console.error(err);
            return;
        }
        console.log("Diagnostics written to file successfully!");
    });
    return filePath;
}