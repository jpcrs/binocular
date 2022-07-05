export const EXTENSION_NAME = 'binocular';

export enum ExternalTerminalCommands {
    windows = `Start-Process PowerShell -ArgumentList "#"`,
    macOs = `osascript -e "tell app \"Terminal\" to activate & do script \"#;exit\""`,
    linux = `x-terminal-emulator -- sh -c "#"`
}