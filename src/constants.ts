export const EXTENSION_NAME = 'binocular';

export enum ExternalTerminalCommands {
    windows = `start cmd /k "# & exit /s"`,
    macOs = `osascript -e 'tell app "Terminal" to do script "ls" & activate & do script "#;exit"`,
    linux = `gnome-terminal -- sh -c "#"`
}