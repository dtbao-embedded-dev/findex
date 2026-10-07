import { invoke } from "@tauri-apps/api/core";

/** Tells Rust the main window is ready: the splash closes and main is shown. */
export function finishSplash(): Promise<void> {
  return invoke<void>("splash_ready");
}
