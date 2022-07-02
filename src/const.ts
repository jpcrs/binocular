export const EXTENSION_NAME = 'binocular';

export enum ExternalTerminalCommands {
    Windows = `start cmd /k "# & exit /s"`,
    Linux = `osascript -e 'tell app "Terminal" to do script "ls" & activate & do script "#;exit"`,
    MacOs = `gnome-terminal -- sh -c "#"`
}