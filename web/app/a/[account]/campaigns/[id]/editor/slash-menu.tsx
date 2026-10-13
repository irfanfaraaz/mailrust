"use client";
import { computePosition, flip, offset, shift } from "@floating-ui/dom";
import { Extension, type Editor } from "@tiptap/core";
import Suggestion, { type SuggestionKeyDownProps, type SuggestionProps } from "@tiptap/suggestion";
import { cn } from "cn";
import { createRoot, type Root } from "react-dom/client";
import { filterBlocks, type BlockId, type BlockItem } from "@/lib/block-menu";

export type DialogKind = "image" | "button" | "merge";

/** Turns the current block into `id`, or opens the dialog for blocks that need input. */
export function runBlock(editor: Editor, id: BlockId, open: (k: DialogKind) => void) {
  const c = editor.chain().focus();
  switch (id) {
    case "paragraph": return c.setParagraph().run();
    case "h1": return c.setHeading({ level: 1 }).run();
    case "h2": return c.setHeading({ level: 2 }).run();
    case "h3": return c.setHeading({ level: 3 }).run();
    case "bullets": return c.toggleBulletList().run();
    case "numbers": return c.toggleOrderedList().run();
    case "quote": return c.toggleBlockquote().run();
    case "divider": return c.setHorizontalRule().run();
    case "image": case "button": case "merge": return open(id);
  }
}

function Menu({ items, index, pick }: { items: BlockItem[]; index: number; pick: (i: BlockItem) => void }) {
  if (items.length === 0) return <div className="rounded-md border bg-popover px-3 py-2 text-sm text-muted-foreground shadow-md">No blocks match</div>;
  return (
    <div role="listbox" aria-label="Blocks" className="w-64 rounded-md border bg-popover p-1 text-sm shadow-md">
      {items.map((it, i) => (
        <button
          key={it.id}
          type="button"
          role="option"
          aria-selected={i === index}
          onMouseDown={(e) => { e.preventDefault(); pick(it); }}
          className={cn("flex w-full flex-col items-start rounded px-2 py-1.5 text-left", i === index && "bg-muted")}
        >
          <span className="font-medium">{it.title}</span>
          <span className="text-xs text-muted-foreground">{it.hint}</span>
        </button>
      ))}
    </div>
  );
}

function menuRenderer() {
  let root: Root | null = null;
  let el: HTMLDivElement | null = null;
  let props: SuggestionProps<BlockItem> | null = null;
  let index = 0;
  const paint = () => {
    if (!root || !props || !el) return;
    const p = props;
    root.render(<Menu items={p.items} index={index} pick={(i) => p.command(i)} />);
    const rect = p.clientRect?.();
    if (rect) {
      const box = el;
      computePosition({ getBoundingClientRect: () => rect }, box, { strategy: "fixed", placement: "bottom-start", middleware: [offset(6), flip(), shift({ padding: 8 })] })
        .then(({ x, y }) => Object.assign(box.style, { left: `${x}px`, top: `${y}px` }));
    }
  };
  return {
    onStart: (p: SuggestionProps<BlockItem>) => {
      props = p;
      index = 0;
      el = document.createElement("div");
      Object.assign(el.style, { position: "fixed", zIndex: "50", left: "0px", top: "0px" });
      document.body.appendChild(el);
      root = createRoot(el);
      paint();
    },
    onUpdate: (p: SuggestionProps<BlockItem>) => {
      props = p;
      index = Math.min(index, Math.max(p.items.length - 1, 0));
      paint();
    },
    onKeyDown: ({ event }: SuggestionKeyDownProps) => {
      if (!props || props.items.length === 0) return false;
      const n = props.items.length;
      if (event.key === "ArrowDown") { index = (index + 1) % n; paint(); return true; }
      if (event.key === "ArrowUp") { index = (index - 1 + n) % n; paint(); return true; }
      if (event.key === "Enter") { props.command(props.items[index]); return true; }
      return false; // Escape: the suggestion plugin closes itself
    },
    onExit: () => {
      root?.unmount();
      el?.remove();
      root = null;
      el = null;
      props = null;
    },
  };
}

/** Typing "/" opens the block menu. `open` shows the dialog for image, button and merge tag. */
export const SlashCommand = Extension.create<{ open: (k: DialogKind) => void }>({
  name: "slashCommand",
  addOptions() {
    return { open: () => {} };
  },
  addProseMirrorPlugins() {
    const open = this.options.open;
    return [
      Suggestion<BlockItem>({
        editor: this.editor,
        char: "/",
        items: ({ query }) => filterBlocks(query),
        command: ({ editor, range, props }) => {
          editor.chain().focus().deleteRange(range).run();
          runBlock(editor, props.id, open);
        },
        render: menuRenderer,
      }),
    ];
  },
});
