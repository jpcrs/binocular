export const EXTENSION_NAME = 'binocular';

export enum ExternalTerminalCommands {
    windows = `start cmd /k "# & exit /s"`,
    linux = `osascript -e 'tell app "Terminal" to do script "ls" & activate & do script "#;exit"`,
    macOs = `gnome-terminal -- sh -c "#"`
}