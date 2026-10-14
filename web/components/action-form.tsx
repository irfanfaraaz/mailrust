"use client";
import { useRouter } from "next/navigation";
import { toast } from "sonner";

/** What a server action may return: an error to show, or a page to go to next. */
type Result = { error?: string; goto?: string } | void | undefined;

type Props = { action: (form: FormData) => unknown; done?: string; className?: string; children: React.ReactNode };

/** A form for a server action: toasts its `{ error }` or the `done` message, then follows `{ goto }`. */
export function ActionForm({ action, done, className, children }: Props) {
  const router = useRouter();
  const run = async (form: FormData) => {
    const r = (await action(form)) as Result;
    if (r?.error) return void toast.error(r.error);
    if (done) toast.success(done);
    if (r?.goto) router.push(r.goto);
  };
  return <form className={className} action={run}>{children}</form>;
}
