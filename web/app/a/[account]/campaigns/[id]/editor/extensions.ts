import { Node } from "@tiptap/core";
import Image from "@tiptap/extension-image";
import { mergeTagLabel } from "@/lib/merge-tags";

/** { type: "mergeTag", attrs: { key, fallback } }: the engine fills it per reader. */
export const MergeTag = Node.create({
  name: "mergeTag",
  group: "inline",
  inline: true,
  atom: true,
  selectable: true,
  addAttributes() {
    return {
      key: { default: "first_name", parseHTML: (el) => el.getAttribute("data-key"), renderHTML: (a) => ({ "data-key": a.key }) },
      fallback: { default: "", parseHTML: (el) => el.getAttribute("data-fallback") ?? "", renderHTML: (a) => ({ "data-fallback": a.fallback }) },
    };
  },
  parseHTML() {
    return [{ tag: "span[data-merge-tag]" }];
  },
  renderHTML({ node, HTMLAttributes }) {
    return ["span", { ...HTMLAttributes, "data-merge-tag": "", class: "merge-tag" }, mergeTagLabel(node.attrs.key, node.attrs.fallback)];
  },
  renderText({ node }) {
    return mergeTagLabel(node.attrs.key, node.attrs.fallback);
  },
});

/** { type: "button", attrs: { label, href } }: the engine draws a table "bulletproof" button. */
export const EmailButton = Node.create({
  name: "button",
  group: "block",
  atom: true,
  draggable: true,
  addAttributes() {
    return { label: { default: "Read more" }, href: { default: "https://" } };
  },
  parseHTML() {
    return [{ tag: "div[data-email-button]", getAttrs: (el) => ({ label: el.getAttribute("data-label") ?? "", href: el.getAttribute("data-href") ?? "" }) }];
  },
  renderHTML({ node }) {
    return ["div", { "data-email-button": "", "data-label": node.attrs.label, "data-href": node.attrs.href, class: "email-button" }, ["span", {}, node.attrs.label]];
  },
});

/**
 * { type: "image", attrs: { id, alt, href } }: `src` is always derived from the id. Only our own
 * images parse, so a pasted <img src="https://elsewhere"> is dropped instead of breaking autosave.
 */
export const CampaignImage = Image.extend({
  addAttributes() {
    return { id: { default: null }, alt: { default: "" }, href: { default: null } };
  },
  parseHTML() {
    return [{ tag: "img[data-image-id]", getAttrs: (el) => ({ id: el.getAttribute("data-image-id"), alt: el.getAttribute("alt") ?? "", href: el.getAttribute("data-href") }) }];
  },
  renderHTML({ node }) {
    return ["img", { src: `/img/${node.attrs.id}`, alt: node.attrs.alt, "data-image-id": node.attrs.id, "data-href": node.attrs.href ?? undefined }];
  },
});
