"use client";
import { useActionState } from "react";
import { NativeSelect } from "@/components/native-select";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import type { Schemas } from "@/lib/engine";
import { uploadImport, type UploadState } from "./actions";

export function UploadForm({ account, tags }: { account: string; tags: Schemas["TagView"][] }) {
  const [state, action, pending] = useActionState<UploadState, FormData>(uploadImport.bind(null, account), {});
  return (
    <Card className="max-w-xl">
      <CardHeader>
        <h2 className="text-lg leading-snug font-semibold">Upload a CSV</h2>
      </CardHeader>
      <CardContent>
    <form action={action} className="flex flex-col gap-4 text-sm">
      <label className="flex flex-col gap-1.5"><span className="label-caps text-muted-foreground">CSV file</span>
        <input type="file" name="file" accept=".csv,text/csv" required className="text-sm" />
      </label>
      <p className="text-muted-foreground">
        A header row with an <code className="font-mono text-foreground">email</code> column. Also read: <code className="font-mono text-foreground">first_name</code>, <code className="font-mono text-foreground">last_name</code> (or <code className="font-mono text-foreground">name</code>),
        and <code className="font-mono text-foreground">tags</code> (split by ; or ,). Every other column becomes a field. Up to 20 MB and 100,000 rows.
      </p>
      <label className="flex flex-col gap-1.5"><span className="label-caps text-muted-foreground">Tag every row</span>
        <NativeSelect name="tag_id" defaultValue="">
          <option value="">No tag</option>
          {tags.map((t) => <option key={t.id} value={t.id}>{t.name}</option>)}
        </NativeSelect>
      </label>
      <label className="flex items-start gap-2">
        <input type="checkbox" name="consent" required className="mt-0.5 size-4 accent-primary" />
        <span>These people agreed to get my emails</span>
      </label>
      <p className="text-muted-foreground">New addresses join as subscribed. Existing ones get their names, fields and tags updated; anyone who unsubscribed stays unsubscribed.</p>
      {state.error && <p role="alert" className="text-destructive">{state.error}</p>}
      <Button type="submit" disabled={pending} className="self-start">{pending ? "Uploading…" : "Import"}</Button>
    </form>
      </CardContent>
    </Card>
  );
}
