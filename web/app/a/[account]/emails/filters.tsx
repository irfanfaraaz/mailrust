import { NativeSelect } from "@/components/native-select";
import { statusLabel } from "@/components/status-badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import type { Schemas } from "@/lib/engine";

const STATUSES: Schemas["EmailStatus"][] = ["queued", "sending", "unknown", "sent", "delivered", "delivery_delayed", "bounced", "complained", "failed", "suppressed", "canceled"];
const PRIORITIES: Schemas["Priority"][] = ["urgent", "normal", "bulk"];

type Values = { status?: string; priority?: string; q?: string; since?: string; until?: string };

/** A plain GET form: filters live in the URL and need no JS. */
export function Filters({ values }: { values: Values }) {
  return (
    <Card size="sm">
    <CardContent>
    <form method="get" className="flex flex-wrap items-end gap-3 text-sm">
      <label className="flex flex-col gap-1.5"><span className="label-caps text-muted-foreground">Search</span>
        <Input name="q" type="search" placeholder="Recipient or subject" defaultValue={values.q ?? ""} />
      </label>
      <label className="flex flex-col gap-1.5"><span className="label-caps text-muted-foreground">Status</span>
        <NativeSelect name="status" defaultValue={values.status ?? ""}>
          <option value="">Any</option>
          {STATUSES.map((s) => <option key={s} value={s}>{statusLabel(s)}</option>)}
        </NativeSelect>
      </label>
      <label className="flex flex-col gap-1.5"><span className="label-caps text-muted-foreground">Priority</span>
        <NativeSelect name="priority" defaultValue={values.priority ?? ""}>
          <option value="">Any</option>
          {PRIORITIES.map((p) => <option key={p} value={p}>{statusLabel(p)}</option>)}
        </NativeSelect>
      </label>
      <label className="flex flex-col gap-1.5"><span className="label-caps text-muted-foreground">From</span>
        <Input name="since" type="date" defaultValue={values.since ?? ""} />
      </label>
      <label className="flex flex-col gap-1.5"><span className="label-caps text-muted-foreground">To</span>
        <Input name="until" type="date" defaultValue={values.until ?? ""} />
      </label>
      <Button type="submit" variant="outline">Filter</Button>
    </form>
    </CardContent>
    </Card>
  );
}
