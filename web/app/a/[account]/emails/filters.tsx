import { NativeSelect } from "@/components/native-select";
import { statusLabel } from "@/components/status-badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { Schemas } from "@/lib/engine";

const STATUSES: Schemas["EmailStatus"][] = ["queued", "sending", "unknown", "sent", "delivered", "delivery_delayed", "bounced", "complained", "failed", "suppressed", "canceled"];
const PRIORITIES: Schemas["Priority"][] = ["urgent", "normal", "bulk"];

type Values = { status?: string; priority?: string; q?: string; since?: string; until?: string };

/** A plain GET form: filters live in the URL and need no JS. */
export function Filters({ values }: { values: Values }) {
  return (
    <form method="get" className="flex flex-wrap items-end gap-3 text-sm">
      <label className="flex flex-col gap-1">Search
        <Input name="q" type="search" placeholder="Recipient or subject" defaultValue={values.q ?? ""} />
      </label>
      <label className="flex flex-col gap-1">Status
        <NativeSelect name="status" defaultValue={values.status ?? ""}>
          <option value="">Any</option>
          {STATUSES.map((s) => <option key={s} value={s}>{statusLabel(s)}</option>)}
        </NativeSelect>
      </label>
      <label className="flex flex-col gap-1">Priority
        <NativeSelect name="priority" defaultValue={values.priority ?? ""}>
          <option value="">Any</option>
          {PRIORITIES.map((p) => <option key={p} value={p}>{statusLabel(p)}</option>)}
        </NativeSelect>
      </label>
      <label className="flex flex-col gap-1">From
        <Input name="since" type="date" defaultValue={values.since ?? ""} />
      </label>
      <label className="flex flex-col gap-1">To
        <Input name="until" type="date" defaultValue={values.until ?? ""} />
      </label>
      <Button type="submit" variant="outline">Filter</Button>
    </form>
  );
}
