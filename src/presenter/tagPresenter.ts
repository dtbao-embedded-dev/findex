import {
  addTag,
  createTag,
  deleteTag,
  removeTag,
  tagCatalog,
  type TagInfo,
} from "../model/tags";

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

/** The tag catalog; none when it cannot be read, since tags only add shortcuts. */
export async function loadTags(
  list: () => Promise<TagInfo[]> = tagCatalog,
): Promise<TagInfo[]> {
  try {
    return await list();
  } catch {
    return [];
  }
}

/**
 * Custom tags the folder lacks, starting with what is typed, in catalog order. The
 * colour tags are offered as their own row, so they are not suggested here.
 */
export function suggestTags(all: TagInfo[], current: string[], typed: string): string[] {
  const start = typed.trim().replace(/^#/, "").toLowerCase();
  return all
    .filter((t) => !t.preset)
    .map((t) => t.tag)
    .filter((tag) => !current.includes(tag) && tag.startsWith(start))
    .slice(0, 8);
}

/** The colour of each colour tag, by name, for drawing a tag found by name alone. */
export function tagColors(all: TagInfo[]): Record<string, string> {
  return Object.fromEntries(all.flatMap((t) => (t.color === null ? [] : [[t.tag, t.color]])));
}

/** A tag's name beside its mark: a colour tag by its colour ("Red"), a custom one as is. */
export function tagName(tag: string, colors: Record<string, string>): string {
  return tag in colors ? tag.charAt(0).toUpperCase() + tag.slice(1) : tag;
}

/** How a tag is written on its own: a colour tag as "Red", a custom one as "#fw". */
export function tagLabel(tag: string, colors: Record<string, string>): string {
  return tag in colors ? tagName(tag, colors) : `#${tag}`;
}

/** The filter tags the catalog still has; a deleted tag must stop narrowing. */
export function keepKnown(tags: string[], all: TagInfo[]): string[] {
  return tags.filter((tag) => all.some((t) => t.tag === tag));
}

/** The question asked before a custom tag is deleted. */
export function deletePrompt(info: TagInfo): string {
  const loses =
    info.folders === 0
      ? ""
      : ` It comes off ${info.folders} folder${info.folders === 1 ? "" : "s"}.`;
  return `Delete #${info.tag}?${loses}`;
}

/** The catalog after a change (null when unchanged), or why the change failed. */
export type CatalogEdit = { catalog: TagInfo[] | null; error: string | null };

/** Creates a custom tag from what was typed; an empty entry is ignored. */
export async function createTagNamed(
  text: string,
  create: (tag: string) => Promise<TagInfo[]> = createTag,
): Promise<CatalogEdit> {
  const tag = text.trim();
  if (tag === "") {
    return { catalog: null, error: null };
  }
  try {
    return { catalog: await create(tag), error: null };
  } catch (err) {
    return { catalog: null, error: String(err) };
  }
}

export async function deleteTagNamed(
  tag: string,
  remove: (tag: string) => Promise<TagInfo[]> = deleteTag,
): Promise<CatalogEdit> {
  try {
    return { catalog: await remove(tag), error: null };
  } catch (err) {
    return { catalog: null, error: String(err) };
  }
}
