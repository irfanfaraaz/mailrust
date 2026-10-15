import type { ReactNode } from "react";
import { Martian_Mono, Schibsted_Grotesk, Silkscreen } from "next/font/google";
import { BounceDemo, CopyButton, SendChecks, SlashMenu, Tabs, Tagline, WavePlan } from "./demos";
import { HeroScene } from "./hero-scene";
import { LandingMotion } from "./motion";
import { ThemeToggle, themeScript } from "./theme-toggle";
import "./landing.css";

// Only the hero's pixel drawing uses it.
const silkscreen = Silkscreen({ weight: "400", subsets: ["latin"], variable: "--font-silkscreen" });
// The landing's own faces; the dashboard keeps Geist.
const schibsted = Schibsted_Grotesk({ subsets: ["latin"], variable: "--font-schibsted" });
const martian = Martian_Mono({ subsets: ["latin"], axes: ["wdth"], variable: "--font-martian" });

const REPO = "https://github.com/irfanfaraaz/mailrust";
const DOCS = `${REPO}#readme`;
const VERSION = "0.3.1";
const CLONE = `git clone ${REPO}`;

const LOCAL = ["git clone https://github.com/irfanfaraaz/mailrust", "cd mailrust && cp .env.example .env", "docker compose up -d postgres", "cargo run -- migrate", "cargo run -- serve"];
const PROD = ["cd deploy", "docker compose -f docker-compose.prod.yml \\\n    --env-file ../.env up -d"];

const Arr = () => (
  <span className="arr" aria-hidden="true">
    &gt;
  </span>
);

const Win = ({ title, children, className }: { title: string; children: ReactNode; className?: string }) => (
  <div className={className ? `win ${className}` : "win"}>
    <div className="bar">
      <i className="x" />
      <span className="t">{title}</span>
    </div>
    {children}
  </div>
);

const features: { group: string; lead: string; items: ReactNode[] }[] = [
  {
    group: "Write",
    lead: "The newsletter editor.",
    items: [
      "Block editor with headings, lists, quotes, images, buttons and dividers",
      <>Type <code>/</code> to add any block</>,
      "Merge tags with fallbacks, for when a reader has no first name",
      "Autosave while you type",
      "Desktop and mobile preview",
      "Test sends to your team before the real one",
      "Image uploads: PNG, JPEG, GIF and WebP up to 5 MB",
      "Public archive of past newsletters, served with a strict content policy",
      'Postal address and an "ask for first name" switch in settings',
      "Delete a draft you no longer need",
    ],
  },
  {
    group: "Audience",
    lead: "Your subscribers, in your database.",
    items: [
      "Contacts with status, first and last name and your own custom fields",
      "Tags, to send to part of your list",
      "Hosted signup form with double opt-in",
      "An embed snippet for the signup form",
      "Rate limits per address and per IP on the form",
      "The form answers the same for every address, so nobody can probe your list",
      "One-link unsubscribe, plus one-click unsubscribe from the inbox (RFC 8058)",
      "CSV import of up to 100,000 rows that picks up again after a crash",
      "CSV export that defuses spreadsheet formulas",
    ],
  },
  {
    group: "Send safely",
    lead: 'The checks between "Send" and the inbox.',
    items: [
      "Waves of 100, 200, 400 and up to 10,000, ten minutes apart",
      "Most engaged readers get each newsletter first",
      "Send to everyone or to chosen tags, now or on a schedule",
      "Checks before every send: verified sender, subject, postal address, size and valid blocks",
      "Warns before Gmail's 100 KB clip cuts your newsletter short",
      "Auto-pause when bounces pass 2% or complaints pass 0.08%",
      "Pause, resume or cancel a send at any point",
      "Nobody gets a newsletter twice, even across a restart",
      "One block list per account, shared by the newsletter and the API",
      "Unsubscribe headers on every newsletter",
    ],
  },
  {
    group: "Sending API",
    lead: "For the email your app sends.",
    items: [
      <><code>POST /emails</code> for one email</>,
      "Batch send of up to 100 emails in one request",
      "Scheduled send up to 30 days ahead, with cancel",
      "Idempotency keys, so a retry never sends twice",
      "Attachments",
      "cc, bcc, reply-to, custom headers and tags",
      "Priority lanes: urgent, normal and bulk",
      "Signed webhooks with retries",
      "Webhooks refuse private network addresses (SSRF guard)",
      "Open and click tracking",
      "OpenAPI reference generated from the code",
    ],
  },
  {
    group: "Deliverability",
    lead: "Your Amazon SES account, handled with care.",
    items: [
      "Each account is its own SES tenant",
      "DKIM and custom MAIL FROM records set up for you",
      "Domains are checked for you until they verify",
      "Send rate follows your SES limits, synced every five minutes",
      "Hitting the SES daily cap pauses sending instead of failing emails",
      "The last 10% of the daily quota is kept for urgent mail",
      "Bounce, complaint, delivery, reject and delay events from SES, signature-checked",
      "Tracking links on your own subdomain, with HTTPS set up for you",
      "A send that times out is never sent a second time",
    ],
  },
  {
    group: "Measure",
    lead: "Numbers you can trust.",
    items: [
      "Report for each newsletter: delivered, opens, clicks and spam complaints",
      "Mail scanner clicks are left out of your open and click rates",
      "Clicks for every link in the newsletter",
      "Split by mailbox provider, such as Gmail or Outlook",
      "Live wave progress and the time of the next wave",
      "Searchable log of every email, filtered by priority and date",
      "Every time shown in the reader's own time zone",
    ],
  },
  {
    group: "Run it",
    lead: "What it takes on your server.",
    items: [
      "Teams with owners and members, and email invites",
      "Passwordless login with magic links",
      "Docker image and a production compose file",
      "Caddy in front, with automatic HTTPS",
      "Nightly Postgres backups",
      <>A fake sender that writes <code>.eml</code> files, so you can try it without AWS</>,
      "A CLI for accounts, API keys, domains and invites",
      "AGPL-3.0 licence",
    ],
  },
];
const featureCount = features.reduce((n, f) => n + f.items.length, 0);

const rust: { k: string; v: ReactNode; proof: string }[] = [
  { k: "Checked SQL", v: <>Every database query is checked against the real schema when the code compiles. <b>A typo in a column name fails the build</b> before it can fail a send.</>, proof: "sqlx query! macros" },
  { k: "Memory safety", v: <>The engine has <b>zero <code>unsafe</code> blocks</b>. There is no garbage collector, so a large send has no pauses in the middle.</>, proof: "unsafe: 0" },
  { k: "One process", v: <>One async process runs the API, the send worker, the newsletter waves, webhook delivery, CSV imports and the SES quota sync. <b>Postgres is the queue</b>, so there is no Redis to run.</>, proof: "tokio · SKIP LOCKED" },
  { k: "Strict input", v: <>The API rejects fields it doesn&apos;t know. Send <code>&quot;prority&quot;</code> by mistake and you get an error back, <b>not a silently ignored setting</b>.</>, proof: "deny_unknown_fields" },
  { k: "Modern TLS", v: <>Connections use rustls, a TLS library written in Rust. <b>There is no OpenSSL</b> in the engine to patch.</>, proof: "rustls" },
  { k: "Strict CI", v: <>Every change passes Clippy with warnings treated as errors, and cargo-deny checks each dependency against the <b>RustSec advisory list</b>.</>, proof: "clippy -D warnings · cargo-deny" },
  { k: "Typed API docs", v: <>The OpenAPI reference is generated from the same Rust types the API uses, so <b>the docs match the code</b>.</>, proof: "GET /openapi.json" },
];

const releases = [
  { v: "0.3.1", date: "Oct 14, 2026", text: "Newsletter reports with spam complaints, local time zones everywhere, toasts for every action, and a round of fixes." },
  { v: "0.3.0", date: "Oct 14, 2026", text: "Newsletters: the block editor, waves, checks before send, reports, the public archive and image uploads." },
  { v: "0.2.0", date: "Oct 13, 2026", text: "The dashboard and your audience: contacts, tags, signup form, CSV import and export, unsubscribe." },
  { v: "0.1.0", date: "Oct 13, 2026", text: "The sending API and the engine: queue, SES sender, priority lanes, webhooks, tracking and auto-pause." },
];

const faq = [
  { q: "What do I need to run it?", a: "A server with Docker, a domain you control and an Amazon SES account. Postgres is the only other service, and the production compose file starts it for you along with the dashboard, Caddy and nightly backups." },
  { q: "Do I need my own Amazon SES account?", a: "Yes. Mailrust sends through your SES account, so the sending reputation, the limits and the bill are yours. To try it locally you don't need AWS at all, because the fake sender writes each email to a file." },
  { q: "Can I use only the API, or only the newsletter?", a: "Yes. Each door works on its own. If you use both, they share one block list and one queue, and urgent API mail always goes ahead of a newsletter." },
  { q: "How is this different from Resend or beehiiv?", a: "Resend is a sending API and beehiiv is a newsletter platform, and both run on their own servers. Mailrust does both jobs in one engine that runs on yours, with your SES account and your database." },
  { q: "What happens if a send goes badly?", a: "The first wave is small and goes to your most engaged readers. If bounces pass 2% or complaints pass 0.08%, Mailrust pauses the account and the newsletter stops. When you resume, nobody who already got it gets it again." },
  { q: "Can I bring my subscribers with me?", a: "Yes. Import a CSV of up to 100,000 rows. The import runs in the background and picks up where it stopped if the server restarts. You can export your contacts as CSV at any time." },
  { q: "What does the AGPL licence mean for me?", a: "You can run, change and use Mailrust for free, for any purpose. If you change the code and offer it to other people as a network service, you have to share your changes under the same licence." },
];

const code: { id: string; title: string; body: ReactNode }[] = [
  {
    id: "curl",
    title: "curl",
    body: (
      <>
        <span className="c"># send a login code</span>
        {`
curl https://mail.example.com/emails \\
  -H "Authorization: Bearer $MAILRUST_KEY" \\
  -H "Idempotency-Key: login-ana-4812" \\
  -H "Content-Type: application/json" \\
  -d '{
    "from": "Acme <login@acme.dev>",
    "to": "ana@example.com",
    "subject": "Your login code",
    "text": "Your code is 481 207",
    `}
        <span className="k">&quot;priority&quot;: &quot;urgent&quot;</span>
        {`
  }'`}
      </>
    ),
  },
  {
    id: "node",
    title: "Node",
    body: (
      <>
        <span className="c">{"// send a login code"}</span>
        {`
await fetch("https://mail.example.com/emails", {
  method: "POST",
  headers: {
    Authorization: \`Bearer \${process.env.MAILRUST_KEY}\`,
    "Idempotency-Key": "login-ana-4812",
    "Content-Type": "application/json",
  },
  body: JSON.stringify({
    from: "Acme <login@acme.dev>",
    to: "ana@example.com",
    subject: "Your login code",
    text: "Your code is 481 207",
    `}
        <span className="k">priority: &quot;urgent&quot;,</span>
        {`
  }),
});`}
      </>
    ),
  },
  {
    id: "py",
    title: "Python",
    body: (
      <>
        <span className="c"># send a login code</span>
        {`
requests.post(
    "https://mail.example.com/emails",
    headers={
        "Authorization": f"Bearer {os.environ['MAILRUST_KEY']}",
        "Idempotency-Key": "login-ana-4812",
    },
    json={
        "from": "Acme <login@acme.dev>",
        "to": "ana@example.com",
        "subject": "Your login code",
        "text": "Your code is 481 207",
        `}
        <span className="k">&quot;priority&quot;: &quot;urgent&quot;,</span>
        {`
    },
)`}
      </>
    ),
  },
  {
    id: "rust",
    title: "Rust",
    body: (
      <>
        <span className="c">{"// send a login code"}</span>
        {`
reqwest::Client::new()
    .post("https://mail.example.com/emails")
    .bearer_auth(std::env::var("MAILRUST_KEY")?)
    .header("Idempotency-Key", "login-ana-4812")
    .json(&json!({
        "from": "Acme <login@acme.dev>",
        "to": "ana@example.com",
        "subject": "Your login code",
        "text": "Your code is 481 207",
        `}
        <span className="k">&quot;priority&quot;: &quot;urgent&quot;</span>
        {`
    }))
    .send()
    .await?;`}
      </>
    ),
  },
];

const Term = ({ lines, comments = [] }: { lines: string[]; comments?: string[] }) => (
  <pre className="term">
    {comments.map((c) => (
      <span key={c} className="c">
        {c}
        {"\n"}
      </span>
    ))}
    {lines.map((l, i) => (
      <span key={l}>
        <span className="p">$ </span>
        {l}
        {i < lines.length - 1 ? "\n" : null}
      </span>
    ))}
  </pre>
);

const newsletterDoor = (
  <div className="door">
    <div className="copytext">
      <h3>Write a newsletter, send it in waves, read the report</h3>
      <p className="lede">
        Write in a block editor and type <kbd>/</kbd> to add images, buttons and merge tags. Before it sends, Mailrust checks the newsletter. Then it mails your most engaged readers first and grows each wave from there.
      </p>
      <ul className="ticks">
        <li>Block editor with autosave, previews and test sends to your team</li>
        <li>Signup form with double opt-in, ready to embed on your site</li>
        <li>Growing waves that stop on their own if bounces climb</li>
        <li>A report with opens, clicks per link and a split by mailbox provider</li>
      </ul>
    </div>
    <Win title="Notes from the shop, No. 12" className="editor">
      <div className="body doc">
        <span className="meta">Saved just now · 3.1 KB of Gmail&apos;s 100 KB</span>
        <div className="ttl">We moved the kiln</div>
        <p className="para">
          Hi {"{first_name}"}, the new studio has a door wide enough for the kiln at last. Here is what the first firing looked like.
          <span className="caret" aria-hidden="true" />
        </p>
        <div className="slash" aria-hidden="true">
          <div className="q">/ima</div>
          <SlashMenu
            items={[
              { name: "Heading", hint: "H1 H2 H3" },
              { name: "Image", hint: "PNG JPG GIF WEBP" },
              { name: "Button", hint: "link" },
              { name: "Merge tag", hint: "{first_name}" },
              { name: "Divider", hint: "---" },
            ]}
          />
        </div>
      </div>
      <div className="sendrow">
        <span className="px">Next wave: 400 readers</span>
        <span className="eyebrow">4,812 subscribed · tag: customers</span>
      </div>
    </Win>
  </div>
);

const apiDoor = (
  <div className="door">
    <div className="copytext">
      <h3>One request per email</h3>
      <p className="lede">
        Send login codes, receipts and alerts with <code>POST /emails</code>. Mark them <code>urgent</code> and they skip ahead of any newsletter in the queue. Retry with the same idempotency key and the email still goes out once.
      </p>
      <ul className="ticks">
        <li>Send one email or a batch of up to 100</li>
        <li>Schedule up to 30 days ahead, and cancel before it leaves</li>
        <li>Attachments, cc, bcc, reply-to and your own headers</li>
        <li>Signed webhooks for delivered, bounced and complained</li>
      </ul>
    </div>
    <Win title="POST /emails">
      <Tabs label="Code language" className="langs" tabs={code.map((c) => ({ id: `code-${c.id}`, title: c.title, panel: <pre className="code">{c.body}</pre> }))} />
      <div className="resp">
        <span className="ok">200</span>
        <code>{'{ "id": "0192f3c1-…", "priority": "urgent" }'}</code>
      </div>
    </Win>
  </div>
);

/** The public front page. Only shown on our own deploy (MAILRUST_LANDING=true) to visitors who are not logged in. */
export function Landing() {
  return (
    // The inline script may set data-theme before React hydrates.
    <div className={`mr-landing ${silkscreen.variable} ${schibsted.variable} ${martian.variable}`} suppressHydrationWarning>
      <script dangerouslySetInnerHTML={{ __html: themeScript }} />
      <LandingMotion />
      <header className="nav">
        <div className="wrap">
          <a className="logo" href="#top" aria-label="Mailrust home">
            <svg viewBox="0 0 12 9" aria-hidden="true" shapeRendering="crispEdges">
              <path fill="currentColor" d="M0 0h12v1H0zM0 8h12v1H0zM0 1h1v7H0zM11 1h1v7h-1zM1 1h1v1H1zM2 2h1v1H2zM3 3h1v1H3zM4 4h1v1H4zM5 5h2v1H5zM7 4h1v1H7zM8 3h1v1H8zM9 2h1v1H9zM10 1h1v1h-1z" />
            </svg>
            Mailrust
          </a>
          <ul>
            <li><a href="#doors">Product</a></li>
            <li><a href="#features">Features</a></li>
            <li><a href="#how">How it works</a></li>
            <li><a href="#faq">FAQ</a></li>
            <li><a href={REPO}>GitHub</a></li>
          </ul>
          <ThemeToggle />
          <a className="btn pri" href="#run">Self-host</a>
        </div>
      </header>

      <main id="top">
        <section className="hero">
          <div className="wrap">
            <span className="pill"><b>v{VERSION}</b> Self-hosted · Open source · Written in Rust · AGPL-3.0</span>
            <h1><span className="hl">One server</span> for every email you send.</h1>
            <p className="lede">
              Mailrust is an open-source email platform you run on your own server. Write newsletters for your contacts in a dashboard, and send transactional emails from your app with an API. Every email goes out through your own Amazon SES account.
            </p>
            <div className="ctas">
              <a className="btn pri" href={REPO}>Self-host Mailrust <Arr /></a>
              <a className="btn" href={DOCS}>Read the docs</a>
            </div>
            <div className="clone">
              <span>{CLONE}</span>
              <CopyButton className="copy" text={CLONE} />
            </div>
            <HeroScene />
            <ul className="strip">
              <li>Written in Rust</li>
              <li>One other service to run: Postgres</li>
              <li>AGPL-3.0, every line public</li>
            </ul>
          </div>
        </section>

        <section id="doors">
          <div className="wrap">
            <Tabs
              label="Choose a product"
              className="tabs"
              before={
                <div className="head">
                  <span className="eyebrow">One server, two jobs</span>
                  <h2>Newsletters and transactional emails, from one place</h2>
                  <p className="lede">
                    Newsletters go out in bulk to a contact list. Transactional emails are personal and triggered by your app: login codes, receipts, alerts. Mailrust sends both from one queue and one block list, so a big newsletter never holds up a login code.
                  </p>
                </div>
              }
              tabs={[
                { id: "news", title: "Newsletters", panel: newsletterDoor },
                { id: "api", title: "Transactional email", panel: apiDoor },
              ]}
            />
          </div>
        </section>

        <section className="tagline" aria-label="What you own">
          <div className="wrap">
            <Tagline text="Your list stays in your database. Your mail goes out through your SES account. Nothing in between is hidden." />
          </div>
        </section>

        <section id="rust">
          <div className="wrap">
            <div className="head">
              <span className="eyebrow">Why Rust</span>
              <h2>Built in Rust, so the compiler catches what would hurt a send</h2>
              <p className="lede">An email platform runs for months and must never drop a message or send one twice. These are the parts of Mailrust that Rust makes hard to get wrong.</p>
            </div>
            <div className="plate">
              <div className="top"><span>Engine spec</span><span>mailrust v{VERSION}</span></div>
              {rust.map((r) => (
                <div key={r.k} className="row">
                  <span className="k">{r.k}</span>
                  <p className="v">{r.v}</p>
                  <span className="proof">{r.proof}</span>
                </div>
              ))}
            </div>
          </div>
        </section>

        <section id="features">
          <div className="wrap">
            <div className="head">
              <span className="eyebrow">Everything in it</span>
              <div className="count"><b>{featureCount}</b><h2>things Mailrust does today</h2></div>
              <p className="lede">This is the full list for v{VERSION}. Nothing here is a roadmap item.</p>
            </div>
            <div className="catalog">
              {features.map((f) => (
                <Win key={f.group} title={f.group}>
                  <p className="lead">{f.lead}</p>
                  <ul>
                    {f.items.map((it, i) => <li key={i}>{it}</li>)}
                  </ul>
                </Win>
              ))}
            </div>
          </div>
        </section>

        <section id="notes">
          <div className="wrap">
            <div className="head">
              <span className="eyebrow">The hard parts</span>
              <h2>Four things that keep a send from going wrong</h2>
              <p className="lede">Most email trouble starts after you press Send. These are the parts of Mailrust that deal with it, and you can try each one here.</p>
            </div>

            <article className="note">
              <div className="copytext">
                <span className="no">No. 01</span>
                <h3>It sends in waves, best readers first</h3>
                <p className="lede">A newsletter starts with 100 of your most engaged readers. Each wave doubles, ten minutes after the last one, up to 10,000 at a time. If the first wave goes badly, the rest of your list never sees it.</p>
              </div>
              <Win title="Wave plan"><WavePlan /></Win>
            </article>

            <article className="note">
              <div className="copytext">
                <span className="no">No. 02</span>
                <h3>It stops itself when mail starts bouncing</h3>
                <p className="lede">Mailrust watches every bounce and complaint SES reports back. Past 2% bounces or 0.08% complaints, it pauses the account before your sender reputation takes the damage. Drag the slider to see where the line is.</p>
              </div>
              <Win title="Account health"><BounceDemo /></Win>
            </article>

            <article className="note">
              <div className="copytext">
                <span className="no">No. 03</span>
                <h3>Login codes never wait behind a newsletter</h3>
                <p className="lede">Every email has a lane: urgent, normal or bulk. Urgent mail goes first, and the last 10% of your daily SES quota is kept for it. If a big newsletter uses up the day, your password resets still go out.</p>
              </div>
              <Win title="Queue">
                <div className="body lanes" aria-hidden="true">
                  <div className="lane"><span className="px">Urgent</span><div className="q">{[0, 1].map((i) => <i key={i} className="u" />)}</div></div>
                  <div className="lane"><span className="px">Normal</span><div className="q">{[0, 1, 2, 3, 4].map((i) => <i key={i} className="n" />)}</div></div>
                  <div className="lane"><span className="px">Bulk</span><div className="q">{Array.from({ length: 14 }, (_, i) => <i key={i} />)}</div></div>
                  <div style={{ marginTop: 12 }}>
                    <div className="gline" style={{ marginBottom: 10 }}><span className="eyebrow">Daily SES quota</span><span className="eyebrow">last 10%: urgent only</span></div>
                    <div className="quota"><div className="a" /><div className="b" /></div>
                  </div>
                </div>
              </Win>
            </article>

            <article className="note">
              <div className="copytext">
                <span className="no">No. 04</span>
                <h3>It checks the newsletter before it lets you send</h3>
                <p className="lede">The Send button stays locked until the newsletter passes every check. Untick one to see what happens.</p>
              </div>
              <Win title="Ready to send?"><SendChecks /></Win>
            </article>
          </div>
        </section>

        <section id="how">
          <div className="wrap">
            <div className="head">
              <span className="eyebrow">How it works</span>
              <h2>How an email travels</h2>
              <p className="lede">Your app and your dashboard put email into the same Postgres queue. Mailrust sends it through your SES account and records what happens to each one.</p>
            </div>
            <div className="flow">
              <div className="stack">
                <div className="win"><div className="bar"><span className="t">Your app</span></div><div className="body"><p>Calls <code>POST /emails</code> with an API key.</p></div></div>
                <div className="win"><div className="bar"><span className="t">Your editor</span></div><div className="body"><p>You write and press Send in the dashboard.</p></div></div>
              </div>
              <div className="arrow" aria-hidden="true" />
              <div className="win">
                <div className="bar"><span className="t">Mailrust engine</span></div>
                <div className="body">
                  <p>Checks, queues and paces every email.</p>
                  <div className="parts">
                    {["API", "Postgres queue", "Priority lanes", "Waves", "Rate limiter", "Block list"].map((p) => <span key={p}>{p}</span>)}
                  </div>
                </div>
              </div>
              <div className="arrow" aria-hidden="true" />
              <div className="win"><div className="bar"><span className="t">Your Amazon SES</span></div><div className="body"><p>Signs with your DKIM and sends from your domain.</p></div></div>
              <div className="arrow" aria-hidden="true" />
              <div className="win d12"><div className="bar"><span className="t">Inboxes</span></div><div className="body" style={{ background: "var(--paper)" }}><p>Gmail, Outlook, iCloud and the rest.</p></div></div>
            </div>
            <div className="back">
              <span className="px">The way back</span>
              <p>SES reports every delivery, bounce and complaint. Mailrust checks the signature, updates the block list, pauses the account if a rate gets too high, fills in your reports and tells your app through a signed webhook.</p>
            </div>
          </div>
        </section>

        <section id="run">
          <div className="wrap">
            <div className="head">
              <span className="eyebrow">Run it yourself</span>
              <h2>Run it on your own server</h2>
              <p className="lede">
                To run Mailrust in production you need a server with Docker, a domain you control and an Amazon SES account. To try it on your laptop you need none of them: until you add SES keys, Mailrust writes each email to a folder as an <code>.eml</code> file.
              </p>
            </div>
            <div className="run">
              <Win title="On your laptop">
                <Term lines={LOCAL} />
                <div className="sendrow"><span className="eyebrow">No AWS needed</span><CopyButton className="btn" text={LOCAL.join("\n")} /></div>
              </Win>
              <Win title="On your server">
                <Term comments={["# Postgres, Mailrust, the dashboard,", "# Caddy with HTTPS and nightly backups"]} lines={PROD} />
                <div className="sendrow"><span className="eyebrow">One small server</span><CopyButton className="btn" text={PROD.join("\n")} /></div>
              </Win>
            </div>
            <div className="need">
              <div><span className="px">You bring</span><p>A server, a domain and an Amazon SES account. New SES accounts start in a sandbox, so ask AWS for production access early.</p></div>
              <div><span className="px">You set up once</span><p>DNS records for DKIM and MAIL FROM. Mailrust gives you the exact records and checks them until they pass.</p></div>
              <div><span className="px">You keep</span><p>Upgrades are a pull and a migrate. Backups run every night on their own.</p></div>
            </div>
          </div>
        </section>

        <section id="proof">
          <div className="wrap">
            <div className="head">
              <span className="eyebrow">Built in the open</span>
              <h2>Every release, and what isn&apos;t in it yet</h2>
              <p className="lede">Mailrust is new. Here is exactly what has shipped, and the things it does not do today, so you can decide with the full picture.</p>
            </div>
            <div className="proof">
              <Win title="What shipped">
                <ul className="rel">
                  {releases.map((r) => (
                    <li key={r.v}><span className="v">{r.v}<small>{r.date}</small></span><p>{r.text}</p></li>
                  ))}
                </ul>
                <div className="sendrow"><span className="eyebrow">AGPL-3.0</span><a className="btn" href={REPO}>See it on GitHub <Arr /></a></div>
              </Win>
              <Win title="What it doesn't do yet">
                <ul className="not">
                  <li>Paid subscriptions for your newsletter</li>
                  <li>Automated email sequences, such as a welcome series</li>
                  <li>Saved email templates on the server. Your code sends the HTML.</li>
                  <li>Receiving email</li>
                  <li>Podcasts, ads or a website builder</li>
                </ul>
              </Win>
            </div>
          </div>
        </section>

        <section id="faq">
          <div className="wrap">
            <div className="head">
              <span className="eyebrow">FAQ</span>
              <h2>Questions people ask first</h2>
            </div>
            <div className="faq">
              {faq.map((f, i) => (
                <details key={f.q} open={i === 0}>
                  <summary>{f.q}</summary>
                  <div className="a"><p>{f.a}</p></div>
                </details>
              ))}
            </div>
          </div>
        </section>

        <section className="final" aria-label="Get started">
          <div className="inner">
            <h2>Your list. Your server. Your send button.</h2>
            <p>Clone the repo and send your first email from your laptop. You don&apos;t need an AWS account to start.</p>
            <div className="ctas">
              <a className="btn pri" href={REPO}>Self-host Mailrust <Arr /></a>
              <a className="btn" href={DOCS}>Read the docs</a>
            </div>
          </div>
        </section>
      </main>

      <footer>
        <div className="wrap">
          <span className="logo">Mailrust</span>
          <span>Open source email, built with Rust and Postgres.</span>
          <nav>
            <a href={REPO}>GitHub</a>
            <a href={DOCS}>Docs</a>
            <a href={`${REPO}/blob/main/CHANGELOG.md`}>Changelog</a>
            <a href={`${REPO}/blob/main/LICENSE`}>AGPL-3.0</a>
          </nav>
        </div>
      </footer>
    </div>
  );
}
