// "Morph from the button you pressed": remember where the pressed thing sat, then let the next screen grow out of it (FLIP).
// A plain transform animation, so it also works when the next page streams in after a loading state.

let from: { key: string; rect: DOMRect; at: number } | null = null;

export function rememberFrom(key: string, el: Element) {
  from = { key, rect: el.getBoundingClientRect(), at: performance.now() };
}

/** Plays once for the matching key. `uniform` keeps text from stretching (same scale on both axes). */
export function morphFrom(key: string, el: HTMLElement, { uniform = false } = {}) {
  const f = from;
  if (!f || f.key !== key) return;
  from = null;
  if (performance.now() - f.at > 4000 || matchMedia("(prefers-reduced-motion: reduce)").matches) return;
  const to = el.getBoundingClientRect();
  if (!to.width || !to.height) return;
  const sy = f.rect.height / to.height;
  const sx = uniform ? sy : f.rect.width / to.width;
  el.animate(
    [
      { transformOrigin: "top left", transform: `translate(${f.rect.left - to.left}px, ${f.rect.top - to.top}px) scale(${sx}, ${sy})`, opacity: 0.5 },
      { transformOrigin: "top left", transform: "none", opacity: 1 },
    ],
    { duration: 420, easing: "cubic-bezier(0.22, 1, 0.36, 1)" },
  );
}
