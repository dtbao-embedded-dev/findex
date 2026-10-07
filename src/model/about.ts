import { invoke } from "@tauri-apps/api/core";

/** Name, version and copyright, read by Rust from the bundle config. */
export type About = {
  name: string;
  version: string;
  copyright: string;
};

export function getAbout(): Promise<About> {
  return invoke<About>("app_about");
}
