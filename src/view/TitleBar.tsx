import iconUrl from "../../src-tauri/icons/icon.svg";
import type { TitleState } from "../presenter/aboutPresenter";

type Props = {
  title: TitleState;
  onMinimize: () => Promise<void>;
  onToggleMaximize: () => Promise<void>;
  onClose: () => Promise<void>;
};

/**
 * The custom title bar that replaces the native one (`decorations: false`): the app
 * icon, its name and version, then the window buttons.
 * Dragging needs no code: `data-tauri-drag-region` makes Tauri move the window.
 */
export function TitleBar({ title, onMinimize, onToggleMaximize, onClose }: Props) {
  return (
    <header className="titlebar" data-tauri-drag-region>
      <span className="titlebar-brand" data-tauri-drag-region>
        <img
          className="titlebar-icon"
          src={iconUrl}
          alt=""
          width={18}
          height={18}
          draggable={false}
          data-tauri-drag-region
        />
        <span className="titlebar-title" data-tauri-drag-region>
          {title.name}
        </span>
        {title.version !== "" && (
          <span className="titlebar-version" data-tauri-drag-region>
            {title.version}
          </span>
        )}
      </span>
      <div className="titlebar-buttons">
        <button type="button" aria-label="Minimize" onClick={() => void onMinimize()}>
          &#x2014;
        </button>
        <button type="button" aria-label="Maximize" onClick={() => void onToggleMaximize()}>
          &#x25A1;
        </button>
        <button type="button" aria-label="Close" onClick={() => void onClose()}>
          &#x2715;
        </button>
      </div>
    </header>
  );
}
