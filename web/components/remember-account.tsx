"use client";
import { useEffect } from "react";

export function RememberAccount({ id }: { id: string }) {
  useEffect(() => {
    document.cookie = `mr_last_account=${id}; path=/; max-age=31536000; samesite=lax`;
  }, [id]);
  return null;
}
