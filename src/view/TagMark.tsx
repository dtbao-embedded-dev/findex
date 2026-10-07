/**
 * How a tag is marked wherever it appears: a preset tag as a dot of its colour, as
 * Finder draws it, and a custom tag as a #.
 */
export function TagMark({ tag, colors }: { tag: string; colors: Record<string, string> }) {
  const color = colors[tag];
  return color === undefined ? (
    <span className="tag-mark" aria-hidden="true">
      #
    </span>
  ) : (
    <span className="tag-dot" data-color={color} aria-hidden="true" />
  );
}
