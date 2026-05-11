import { PlaceholderContext } from "./workspace";

export function expandTemplate(template: string, context: PlaceholderContext): string {
  let result = template;
  result = result.replaceAll("${workspaceFolder}", context.workspaceFolder);
  result = result.replaceAll("${workspaceFolderName}", context.workspaceFolderName);
  result = result.replaceAll("${file}", context.file);
  result = result.replaceAll("${relativeFile}", context.relativeFile);
  result = result.replaceAll("${selectedText}", context.selectedText);
  return result;
}

export function expandArgs(args: string[], context: PlaceholderContext): string[] {
  return args.map((arg) => expandTemplate(arg, context));
}

export function expandEnv(
  env: Record<string, string>,
  context: PlaceholderContext,
): Record<string, string> {
  return Object.fromEntries(
    Object.entries(env).map(([key, value]) => [key, expandTemplate(value, context)]),
  );
}
