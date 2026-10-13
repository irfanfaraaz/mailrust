import { Input } from "@/components/ui/input";
import type { Schemas } from "@/lib/engine";

const STATUSES: Schemas["EmailStatus"][] = ["queued", "sending", "unknown", "sent", "delivered", "delivery_delayed", "bounced", "complained", "failed", "suppressed", "canceled"];
const PRIORITIES: Schemas["Priority"][] = ["urgent", "normal", "bulk"];
const select = "h-8 rounded-lg border border-input bg-transparent px-2 text-sm";

type Values = { status?: string; priority?: string; q?: string; since?: string; until?: string };

/** A plain GET form: filters live in the URL and need no JS. */
export function Filters({ values }: { values: Values }) {
  return (
    <form method="get" className="flex flex-wrap items-end gap-3 text-sm">
      <label className="flex flex-col gap-1">Search
        <Input name="q" type="search" placeholder="Recipient or subject" defaultValue={values.q ?? ""} />
      </label>
      <label className="flex flex-col gap-1">Status
        <select name="status" className={select} defaultValue={values.status ?? ""}>
          <option value="">Any</option>
          {STATUSES.map((s) => <option key={s} value={s}>{s}</option>)}
        </select>
      </label>
      <label className="flex flex-col gap-1">Priority
        <select name="priority" className={select} defaultValue={values.priority ?? ""}>
          <option value="">Any</option>
          {PRIORITIES.map((p) => <option key={p} value={p}>{p}</option>)}
        </select>
      </label>
      <label className="flex flex-col gap-1">From
        <Input name="since" type="date" defaultValue={values.since ?? ""} />
      </label>
      <label className="flex flex-col gap-1">To
        <Input name="until" type="date" defaultValue={values.until ?? ""} />
      </label>
      <button type="submit" className="h-8 rounded-lg border px-3">Filter</button>
    </form>
  );
}
