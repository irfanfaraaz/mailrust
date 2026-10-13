export type BlockId = "paragraph" | "h1" | "h2" | "h3" | "bullets" | "numbers" | "quote" | "image" | "button" | "divider" | "merge";
export type BlockItem = { id: BlockId; title: string; hint: string; keywords: string[] };

/** The "/" menu, in the order it shows. */
export const BLOCK_ITEMS: BlockItem[] = [
  { id: "paragraph", title: "Text", hint: "A plain paragraph", keywords: ["paragraph", "p"] },
  { id: "h1", title: "Heading 1", hint: "A big section title", keywords: ["title", "h1"] },
  { id: "h2", title: "Heading 2", hint: "A medium heading", keywords: ["subtitle", "h2"] },
  { id: "h3", title: "Heading 3", hint: "A small heading", keywords: ["h3"] },
  { id: "bullets", title: "Bulleted list", hint: "A list with dots", keywords: ["ul", "unordered", "list"] },
  { id: "numbers", title: "Numbered list", hint: "A list with 1, 2, 3", keywords: ["ol", "ordered", "list"] },
  { id: "quote", title: "Quote", hint: "An indented quote", keywords: ["blockquote"] },
  { id: "image", title: "Image", hint: "Upload a PNG, JPEG, GIF or WebP", keywords: ["picture", "photo", "img"] },
  { id: "button", title: "Button", hint: "A link that looks like a button", keywords: ["cta", "link"] },
  { id: "divider", title: "Divider", hint: "A line between sections", keywords: ["hr", "line", "separator"] },
  { id: "merge", title: "Merge tag", hint: "The reader's name or a field", keywords: ["name", "personalize", "variable"] },
];

export function filterBlocks(query: string): BlockItem[] {
  const q = query.trim().toLowerCase();
  if (!q) return BLOCK_ITEMS;
  return BLOCK_ITEMS.filter((i) => i.title.toLowerCase().includes(q) || i.keywords.some((k) => k.startsWith(q)));
}
