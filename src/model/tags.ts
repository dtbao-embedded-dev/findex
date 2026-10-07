import { invoke } from "@tauri-apps/api/core";

/** One tag that can be picked, as `TagInfo` in `model/tags.rs` returns it. */
export type TagInfo = {
  tag: string;
  /** The colour a preset tag is drawn in, such as "red"; null for custom tags. */
  color: string | null;
  /** A named tag that ships with the app (Working, Doc, …) and cannot be deleted. */
  preset: boolean;
  /** How many folders carry it. */
  folders: number;
};

/** Tags the folder; Rust validates the tag and returns the folder's tags, sorted. */
export function addTag(path: string, tag: string): Promise<string[]> {
  return invoke<string[]>("add_tag", { path, tag });
}

/** Untags the folder; returns the tags left, sorted. */
export function removeTag(path: string, tag: string): Promise<string[]> {
  return invoke<string[]>("remove_tag", { path, tag });
}

/** Every tag that can be picked: the presets, then custom tags by name. */
export function tagCatalog(): Promise<TagInfo[]> {
  return invoke<TagInfo[]>("tag_catalog");
}

/** Adds a custom tag; Rust refuses an invalid or existing one. Returns the catalog. */
export function createTag(tag: string): Promise<TagInfo[]> {
  return invoke<TagInfo[]>("create_tag", { tag });
}

/** Deletes a custom tag from the catalog and every folder. Returns the catalog. */
export function deleteTag(tag: string): Promise<TagInfo[]> {
  return invoke<TagInfo[]>("delete_tag", { tag });
}
