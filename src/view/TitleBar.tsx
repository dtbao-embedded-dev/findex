type Props = {
  title: string;
  onMinimize: () => Promise<void>;
  onToggleMaximize: () => Promise<void>;
  onClose: () => Promise<void>;
};

/**
 * The custom title bar that replaces the native one (`decorations: false`).
 * Dragging needs no code: `data-tauri-drag-region` makes Tauri move the window.
 */
export function TitleBar({ title, onMinimize, onToggleMaximize, onClose }: Props) {
  return (
    <header className="titlebar" data-tauri-drag-region>
      <span className="titlebar-title" data-tauri-drag-region>
        {title}
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
