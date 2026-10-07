import iconUrl from "../../src-tauri/icons/icon.svg";
import type { TitleState } from "../presenter/aboutPresenter";

type Props = {
  title: TitleState;
  onMinimize: () => Promise<void>;
  onToggleMaximize: () => Promise<void>;
  onClose: () => Promise<void>;
  onSettings: () => void;
};

/**
 * The custom title bar that replaces the native one (`decorations: false`): the app
 * icon, its name and version; the app's own tools; then the window buttons.
 * Dragging needs no code: `data-tauri-drag-region` makes Tauri move the window.
 */
export function TitleBar({
  title,
  onMinimize,
  onToggleMaximize,
  onClose,
  onSettings,
}: Props) {
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
      <div className="titlebar-tools">
        <button
          type="button"
          className="titlebar-tool"
          aria-label="Settings"
          title="Settings"
          onClick={onSettings}
        >
          <svg viewBox="0 0 20 20" aria-hidden="true">
            <path
              d="M10 6.75a3.25 3.25 0 1 0 0 6.5 3.25 3.25 0 0 0 0-6.5zM8.6 2h2.8l.45 2.1 1.5.62 1.8-1.17 1.98 1.98-1.17 1.8.62 1.5L18 9.3v2.8l-2.1.45-.62 1.5 1.17 1.8-1.98 1.98-1.8-1.17-1.5.62L11.4 18H8.6l-.45-2.1-1.5-.62-1.8 1.17-1.98-1.98 1.17-1.8-.62-1.5L1.3 11.4V8.6l2.1-.45.62-1.5-1.17-1.8L4.83 2.87l1.8 1.17 1.5-.62z"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.4"
              strokeLinejoin="round"
            />
          </svg>
        </button>
      </div>
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
