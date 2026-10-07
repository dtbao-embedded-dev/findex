import { invoke } from "@tauri-apps/api/core";

/**
 * Asks Rust for a greeting. The rule lives in `src-tauri/src/model/greeting.rs`;
 * this module is the only place the frontend names the command.
 */
export function greet(name: string): Promise<string> {
  return invoke<string>("greet", { name });
}
