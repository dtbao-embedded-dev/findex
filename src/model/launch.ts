import { invoke } from "@tauri-apps/api/core";

/** Opens the folder in a File Explorer window. */
export function openFolder(path: string): Promise<void> {
  return invoke<void>("open_folder", { path });
}

/** Opens a terminal working in the folder, with the user's normal rights. */
export function openTerminal(path: string): Promise<void> {
  return invoke<void>("open_terminal", { path });
}

/** Opens Claude Code (`claude`) in a terminal working in the folder. */
export function openClaude(path: string): Promise<void> {
  return invoke<void>("open_claude", { path });
}

/** Opens a folder or a file in Visual Studio Code, with the user's normal rights. */
export function openVsCode(path: string): Promise<void> {
  return invoke<void>("open_vscode", { path });
}
