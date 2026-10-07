import { Fragment, useEffect, useLayoutEffect, useRef, useState, type KeyboardEvent } from "react";

import { placeMenu, type MenuAction, type MenuItem } from "../presenter/menuPresenter";
import { useDismiss } from "./useDismiss";

type Props = {
  /** Where it was asked for: the pointer, or the selected row for the keyboard. */
  at: { x: number; y: number };
  items: MenuItem[];
  onPick: (action: MenuAction) => void;
  onClose: () => void;
};

/**
 * A right-click menu drawn by the app, in place of the WebView's. It grows out of
 * the point it was opened at, takes focus so arrows and Enter work at once, and
 * closes on Esc, a click elsewhere, or the window losing focus.
 */
export function ContextMenu({ at, items, onPick, onClose }: Props) {
  const menu = useRef<HTMLDivElement>(null);
  const [place, setPlace] = useState({ left: at.x, top: at.y, origin: "top left" });

  // Measured before paint, so the menu never shows at a spot it then jumps from.
  useLayoutEffect(() => {
    const box = menu.current?.getBoundingClientRect();
    if (box) {
      setPlace(
        placeMenu(at, box, { width: window.innerWidth, height: window.innerHeight }),
      );
    }
  }, [at]);

  useDismiss(menu, onClose);

  useEffect(() => {
    menu.current?.querySelector<HTMLButtonElement>("[role=menuitem]")?.focus();
  }, []);

  const moveFocus = (event: KeyboardEvent<HTMLDivElement>) => {
    const buttons = Array.from(
      menu.current?.querySelectorAll<HTMLButtonElement>("[role=menuitem]") ?? [],
    );
    const at = buttons.indexOf(document.activeElement as HTMLButtonElement);
    const next = {
      ArrowDown: at + 1,
      ArrowUp: at - 1,
      Home: 0,
      End: buttons.length - 1,
    }[event.key];
    if (next === undefined || buttons.length === 0) {
      return;
    }
    event.preventDefault();
    buttons[(next + buttons.length) % buttons.length]?.focus();
  };

  return (
    <div
      ref={menu}
      className="context-menu"
      role="menu"
      style={{ left: place.left, top: place.top, transformOrigin: place.origin }}
      onKeyDown={moveFocus}
    >
      {items.map((item, index) => (
        <Fragment key={item.action}>
          {index > 0 && items[index - 1]?.group !== item.group && (
            <div className="context-separator" role="separator" />
          )}
          <button type="button" role="menuitem" onClick={() => onPick(item.action)}>
            {item.label}
          </button>
        </Fragment>
      ))}
    </div>
  );
}
