import { addTag, listTags, removeTag, type TagCount } from "../model/tags";

/** The folder's tags after an edit (null when unchanged), or why it failed. */
export type TagEdit = { tags: string[] | null; error: string | null };

/**
 * Adds what was typed as a tag. Rust decides what a valid tag is; an empty entry is
 * simply ignored.
 */
export async function addTagTo(
  path: string,
  text: string,
  add: (path: string, tag: string) => Promise<string[]> = addTag,
): Promise<TagEdit> {
  const tag = text.trim();
  if (tag === "") {
    return { tags: null, error: null };
  }
  try {
    return { tags: await add(path, tag), error: null };
  } catch (err) {
    return { tags: null, error: String(err) };
  }
}

export async function removeTagFrom(
  path: string,
  tag: string,
  remove: (path: string, tag: string) => Promise<string[]> = removeTag,
): Promise<TagEdit> {
  try {
    return { tags: await remove(path, tag), error: null };
  } catch (err) {
    return { tags: null, error: String(err) };
  }
}

/** The tags in use; none when they cannot be read, since they only add shortcuts. */
export async function loadTags(
  list: () => Promise<TagCount[]> = listTags,
): Promise<TagCount[]> {
  try {
    return await list();
  } catch {
    return [];
  }
}

/** Tags in use that the folder lacks, starting with what is typed, most used first. */
export function suggestTags(all: TagCount[], current: string[], typed: string): string[] {
  const start = typed.trim().replace(/^#/, "").toLowerCase();
  return all
    .map((t) => t.tag)
    .filter((tag) => !current.includes(tag) && tag.startsWith(start))
    .slice(0, 8);
}
