import { invoke } from "@tauri-apps/api/core";

/** A tag and how many folders carry it. */
export type TagCount = { tag: string; folders: number };

/** Tags the folder; Rust validates the tag and returns the folder's tags, sorted. */
export function addTag(path: string, tag: string): Promise<string[]> {
  return invoke<string[]>("add_tag", { path, tag });
}

/** Untags the folder; returns the tags left, sorted. */
export function removeTag(path: string, tag: string): Promise<string[]> {
  return invoke<string[]>("remove_tag", { path, tag });
}

/** Every tag in use, most used first. */
export function listTags(): Promise<TagCount[]> {
  return invoke<TagCount[]>("list_tags");
}
