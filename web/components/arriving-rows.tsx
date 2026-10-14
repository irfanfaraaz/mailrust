"use client";
import { useEffect } from "react";

/**
 * Rows that were not in the list last time (this tab) slide in with a short navy wash.
 * `ids`: the rows' data-row ids, newest first, comma-joined.
 */
export function ArrivingRows({ ids, scope }: { ids: string; scope: string }) {
  useEffect(() => {
    const list = ids ? ids.split(",") : [];
    const key = `mr_rows_${scope}`;
    let prev: string | null = null;
    try {
      prev = sessionStorage.getItem(key);
      sessionStorage.setItem(key, list[0] ?? "");
    } catch {
      return; // storage blocked: no "new" marks, the log still works
    }
    const seen = prev ? list.indexOf(prev) : -1;
    for (const id of list.slice(0, Math.max(seen, 0))) document.querySelector(`[data-row="${CSS.escape(id)}"]`)?.classList.add("row-arrive");
  }, [ids, scope]);
  return null;
}
