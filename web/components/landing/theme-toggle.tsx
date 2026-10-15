"use client";

const KEY = "mr-landing-theme";

/** Runs before paint (inline script), so a saved dark choice shows without a flash of light. */
export const themeScript = `try{if(localStorage.getItem("${KEY}")==="dark")document.currentScript.parentElement.dataset.theme="dark"}catch(e){}`;

/** Light/dark switch. The theme lives only in data-theme on the page wrapper; CSS picks the icon and label from it. */
export function ThemeToggle() {
  return (
    <button
      type="button"
      className="theme"
      onClick={(e) => {
        const el = e.currentTarget.closest<HTMLElement>(".mr-landing");
        if (!el) return;
        const next = el.dataset.theme === "dark" ? "light" : "dark";
        if (next === "dark") el.dataset.theme = "dark";
        else delete el.dataset.theme;
        try {
          localStorage.setItem(KEY, next);
        } catch {
          // private mode: the choice just isn't remembered
        }
      }}
    >
      <span className="moon">
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path fill="currentColor" d="M6.2 1.3a6.8 6.8 0 1 0 8.5 8.5A5.6 5.6 0 0 1 6.2 1.3Z" />
        </svg>
        <span className="sr-only">Switch to dark mode</span>
      </span>
      <span className="sun">
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <circle cx="8" cy="8" r="3.2" fill="currentColor" />
          <path stroke="currentColor" strokeWidth="1.5" strokeLinecap="square" d="M8 .8v1.8M8 13.4v1.8M.8 8h1.8M13.4 8h1.8M2.9 2.9l1.3 1.3M11.8 11.8l1.3 1.3M2.9 13.1l1.3-1.3M11.8 4.2l1.3-1.3" />
        </svg>
        <span className="sr-only">Switch to light mode</span>
      </span>
    </button>
  );
}
