import { copyText } from "../model/clipboard";
import { openClaude, openFolder, openTerminal, openVsCode } from "../model/launch";
import { revealPath, type Hit } from "../model/search";

export type MenuAction =
  | "claude"
  | "vscode"
  | "folder"
  | "terminal"
  | "reveal"
  | "copyPath"
  | "copyName"
  | "tags";

/** One menu entry; a change of `group` draws a separator before it. */
export type MenuItem = { action: MenuAction; label: string; group: number };

/** The model calls the menu's actions need, replaceable in tests. */
export type ActionDeps = {
  openClaude: (path: string) => Promise<void>;
  openFolder: (path: string) => Promise<void>;
  openTerminal: (path: string) => Promise<void>;
  openVsCode: (path: string) => Promise<void>;
  reveal: (path: string) => Promise<void>;
  copy: (text: string) => Promise<void>;
};

const MODEL: ActionDeps = {
  openClaude,
  openFolder,
  openTerminal,
  openVsCode,
  reveal: revealPath,
  copy: copyText,
};

/**
 * What a right-click on a result offers: opening it (VS Code opens files and
 * folders alike), copying, then a folder's tags.
 */
export function menuFor(hit: Hit): MenuItem[] {
  const vscode: MenuItem = { action: "vscode", label: "Open with VS Code", group: 0 };
  const open: MenuItem[] = hit.isDir
    ? [
        { action: "claude", label: "Open with Claude", group: 0 },
        vscode,
        { action: "folder", label: "Open folder", group: 0 },
        { action: "terminal", label: "Open in Terminal", group: 0 },
      ]
    : [vscode, { action: "reveal", label: "Show in Explorer", group: 0 }];
  return [
    ...open,
    { action: "copyPath", label: "Copy path", group: 1 },
    { action: "copyName", label: "Copy name", group: 1 },
    ...(hit.isDir ? [{ action: "tags" as const, label: "Tags…", group: 2 }] : []),
  ];
}

/**
 * Runs one menu action on `hit`. A copy changes nothing on screen, so it comes back
 * with a notice confirming it; opening shows its own window. "tags" opens the tag
 * editor, which is the view's to show, so it does nothing here.
 */
export async function runAction(
  action: MenuAction,
  hit: Hit,
  deps: ActionDeps = MODEL,
): Promise<{ notice: string | null; error: string | null }> {
  try {
    switch (action) {
      case "claude":
        await deps.openClaude(hit.path);
        return { notice: null, error: null };
      case "vscode":
        await deps.openVsCode(hit.path);
        return { notice: null, error: null };
      case "folder":
        await deps.openFolder(hit.path);
        return { notice: null, error: null };
      case "terminal":
        await deps.openTerminal(hit.path);
        return { notice: null, error: null };
      case "reveal":
        await deps.reveal(hit.path);
        return { notice: null, error: null };
      case "copyPath":
        await deps.copy(hit.path);
        return { notice: "Path copied", error: null };
      case "copyName":
        await deps.copy(hit.name);
        return { notice: "Name copied", error: null };
      case "tags":
        return { notice: null, error: null };
    }
  } catch (err) {
    return { notice: null, error: String(err) };
  }
}

type Point = { x: number; y: number };
type Size = { width: number; height: number };

/**
 * Where a menu opened at `at` goes: down and right of the pointer, flipped at the
 * window's edges, and never outside it. `origin` is the corner at the pointer, so
 * the menu grows out of where it was asked for.
 */
export function placeMenu(
  at: Point,
  menu: Size,
  view: Size,
): { left: number; top: number; origin: string } {
  const flipX = at.x + menu.width > view.width && at.x - menu.width >= 0;
  const flipY = at.y + menu.height > view.height && at.y - menu.height >= 0;
  const left = flipX ? at.x - menu.width : Math.max(0, Math.min(at.x, view.width - menu.width));
  const top = flipY ? at.y - menu.height : Math.max(0, Math.min(at.y, view.height - menu.height));
  return { left, top, origin: `${flipY ? "bottom" : "top"} ${flipX ? "right" : "left"}` };
}
