import Link from "next/link";
import { Stamp } from "@/components/stamp";
import { buttonVariants } from "@/components/ui/button";
import { SortingMachine } from "@/components/landing/sorting-machine";

const REPO = "https://github.com/irfanfaraaz/mailrust";
const DOCS = `${REPO}#readme`;

const features: { group: string; items: string[] }[] = [
  { group: "Write", items: ["Block editor with / commands", "Preview, and send yourself a test", "Public archive of past issues", "A signup form for new readers"] },
  { group: "Audience", items: ["Contacts", "CSV import", "Tags"] },
  {
    group: "Send safely",
    items: [
      "Waves that start with your most engaged readers",
      "Auto-pause when bounces pass 2% or complaints pass 0.08%",
      "One-click unsubscribe in every email",
      "One suppression list for the newsletter and the API",
      "Priority lanes, so a login code never waits behind a newsletter",
    ],
  },
  { group: "Sending API", items: ["API keys", "Batch send, up to 100 per request", "Scheduled send", "Idempotency keys", "Attachments", "Signed webhooks"] },
  {
    group: "Run it",
    items: ["Your own SES account and domains, with DKIM", "Teams with owners and members", "Postgres is the queue, so there is no Redis and nothing else to run", "AGPL licence"],
  },
];

const steps = [
  `git clone ${REPO}.git`,
  "cd mailrust && cp .env.example .env",
  "docker compose up -d postgres",
  "cargo run -- migrate && cargo run -- serve",
];

/** The public front page. Only shown on our own deploy (MAILRUST_LANDING=true) to visitors who are not logged in. */
export function Landing() {
  return (
    <div className="landing flex min-h-dvh flex-col bg-background text-foreground">
      <header className="sticky top-0 z-10 border-b border-border bg-background/90 backdrop-blur-sm">
        <nav className="mx-auto flex h-14 w-full max-w-6xl items-center justify-between px-4 md:px-8">
          <Link href="/" className="font-heading text-lg font-semibold tracking-tight">Mailrust</Link>
          <div className="flex items-center gap-5 text-sm">
            <a href={DOCS} className="landing-link">Docs</a>
            <a href={REPO} className="landing-link">GitHub</a>
          </div>
        </nav>
      </header>

      <main className="flex-1">
        <section className="landing-hero">
          <div className="mx-auto grid w-full max-w-6xl items-center gap-10 px-4 pt-14 pb-16 md:px-8 lg:grid-cols-[minmax(0,5fr)_minmax(0,6fr)] lg:pt-20 lg:pb-24">
            <div className="landing-rise flex flex-col items-start gap-6">
              <h1 className="font-heading text-[clamp(3.25rem,9vw,6rem)] leading-[0.95] font-semibold tracking-[-0.035em] text-balance">
                Email you own.
              </h1>
              <p className="max-w-[34ch] text-lg leading-relaxed text-muted-foreground md:text-xl">
                Send your newsletter and your app&apos;s email from one Rust engine, through your own Amazon SES account.
              </p>
              <div className="flex flex-wrap items-center gap-3">
                <a href={REPO} className={buttonVariants({ size: "lg" })}>Self-host it</a>
                <span aria-disabled="true" className={`${buttonVariants({ size: "lg", variant: "outline" })} cursor-not-allowed gap-2.5 text-muted-foreground`}>
                  Cloud <Stamp tone="info">Coming soon</Stamp>
                </span>
              </div>
              <p className="text-sm text-muted-foreground">Open source under the AGPL. The only other service it needs is Postgres.</p>
            </div>
            <div className="min-w-0">
              <SortingMachine />
            </div>
          </div>
        </section>

        <section className="border-t border-border">
          <div className="mx-auto w-full max-w-6xl px-4 py-16 md:px-8 lg:py-24">
            <h2 className="font-heading text-3xl font-semibold tracking-[-0.02em] md:text-4xl">One engine, two doors.</h2>
            <div className="mt-10 grid gap-10 md:grid-cols-2 md:gap-0 md:divide-x md:divide-border">
              <div className="flex flex-col gap-5 md:pr-10">
                <h3 className="font-heading text-xl font-semibold"><span className="landing-tab" style={{ ["--tab" as string]: "var(--tab-newsletter)" }}>For writing</span></h3>
                <p className="max-w-[44ch] text-muted-foreground">Draft in a block editor and type / to add a block. Send in waves, then see who opened and who clicked.</p>
                <div aria-hidden className="landing-sheet">
                  <p className="font-heading text-lg font-semibold">Notes from the shop, issue 12</p>
                  <p className="mt-2 text-sm text-muted-foreground">This week we moved the kiln. <span className="landing-caret">/</span></p>
                  <ul className="landing-slash mt-3">
                    {["Heading 1", "Image", "Button", "Divider"].map((b, i) => <li key={b} data-on={i === 0 || undefined}>{b}</li>)}
                  </ul>
                </div>
              </div>
              <div className="flex min-w-0 flex-col gap-5 md:pl-10">
                <h3 className="font-heading text-xl font-semibold"><span className="landing-tab" style={{ ["--tab" as string]: "var(--tab-api)" }}>For your code</span></h3>
                <p className="max-w-[44ch] text-muted-foreground">Send login codes, receipts and alerts with one request. Urgent mail goes ahead of the newsletter queue.</p>
                <pre className="landing-code" aria-label="Example request that sends a login code">
                  <code>
                    <span className="c">{"# send a login code"}</span>{"\n"}
                    {"curl https://mail.example.com/emails \\\n"}
                    {"  -H \"Authorization: Bearer $MAILRUST_KEY\" \\\n"}
                    {"  -d '{\n"}
                    {"    \"from\": \"Shop <hi@example.com>\",\n"}
                    {"    \"to\": [\"ana@example.com\"],\n"}
                    {"    \"subject\": \"Your login code\",\n"}
                    {"    \"text\": \"Your code is 481 207\",\n"}
                    {"    "}<span className="k">{"\"priority\": \"urgent\""}</span>{"\n"}
                    {"  }'"}
                  </code>
                </pre>
              </div>
            </div>
          </div>
        </section>

        <section className="border-t border-border bg-sidebar">
          <div className="mx-auto w-full max-w-6xl px-4 py-16 md:px-8 lg:py-24">
            <h2 className="font-heading text-3xl font-semibold tracking-[-0.02em] md:text-4xl">What&apos;s in it</h2>
            <dl className="mt-10 border-t-2 border-foreground">
              {features.map((f) => (
                <div key={f.group} className="grid gap-3 border-b border-border py-6 md:grid-cols-[12rem_1fr] md:gap-8">
                  <dt className="font-heading text-lg font-semibold">{f.group}</dt>
                  <dd>
                    <ul className="landing-items gap-x-8 sm:columns-2">
                      {f.items.map((it) => <li key={it}>{it}</li>)}
                    </ul>
                  </dd>
                </div>
              ))}
            </dl>
          </div>
        </section>

        <section className="border-t border-border">
          <div className="mx-auto grid w-full max-w-6xl gap-10 px-4 py-16 md:px-8 lg:grid-cols-[minmax(0,4fr)_minmax(0,6fr)] lg:items-center lg:py-24">
            <div className="flex flex-col items-start gap-4">
              <h2 className="font-heading text-3xl font-semibold tracking-[-0.02em] md:text-4xl">Run it yourself</h2>
              <p className="max-w-[40ch] text-muted-foreground">Four commands give you a working copy. Until you add SES keys, mail goes to a local outbox folder.</p>
              <p className="text-sm text-muted-foreground">
                The <a href={DOCS} className="landing-link">README</a> has the full steps, including SES setup and deploy.
              </p>
            </div>
            <div className="relative min-w-0">
              <pre className="landing-code landing-term" aria-label="Commands to run Mailrust locally">
                <code>{steps.map((s) => <span key={s} className="block"><span className="p" aria-hidden>$ </span>{s}</span>)}</code>
              </pre>
              <span className="absolute -top-3 right-4"><Stamp tone="good">Postgres only</Stamp></span>
            </div>
          </div>
        </section>
      </main>

      <footer className="border-t-2 border-foreground">
        <div className="mx-auto flex w-full max-w-6xl flex-col gap-4 px-4 py-8 text-sm text-muted-foreground md:flex-row md:items-center md:justify-between md:px-8">
          <p>Mailrust is open source, built on Rust and Postgres.</p>
          <div className="flex gap-5">
            <a href={REPO} className="landing-link">GitHub</a>
            <a href={DOCS} className="landing-link">Docs</a>
            <a href={`${REPO}/blob/main/LICENSE`} className="landing-link">AGPL-3.0</a>
          </div>
        </div>
      </footer>
    </div>
  );
}
