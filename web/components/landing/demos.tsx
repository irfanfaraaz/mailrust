"use client";

import { useEffect, useRef, useState, useSyncExternalStore, type KeyboardEvent, type ReactNode } from "react";

const fmt = (n: number) => n.toLocaleString("en-US");

const motionQuery = "(prefers-reduced-motion: reduce)";
/** True when the visitor asked for less motion. */
export function useStill() {
  return useSyncExternalStore(
    (cb) => {
      const m = matchMedia(motionQuery);
      m.addEventListener("change", cb);
      return () => m.removeEventListener("change", cb);
    },
    () => matchMedia(motionQuery).matches,
    () => false,
  );
}

export function CopyButton({ text, className, children = "Copy" }: { text: string; className: string; children?: string }) {
  const [label, setLabel] = useState(children);
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
  useEffect(() => () => clearTimeout(timer.current), []);
  return (
    <button
      type="button"
      className={className}
      onClick={async () => {
        try {
          await navigator.clipboard.writeText(text);
          setLabel("Copied");
        } catch {
          setLabel("Copy failed");
        }
        clearTimeout(timer.current);
        timer.current = setTimeout(() => setLabel(children), 1400);
      }}
    >
      {label}
    </button>
  );
}

/** A tab list with arrow-key movement. Each tab shows one panel; the others stay in the DOM but hidden. */
export function Tabs({
  label,
  className,
  tabs,
  before,
}: {
  label: string;
  className: string;
  tabs: { id: string; title: string; panel: ReactNode }[];
  before?: ReactNode;
}) {
  const [active, setActive] = useState(tabs[0].id);
  const refs = useRef<(HTMLButtonElement | null)[]>([]);
  const onKey = (e: KeyboardEvent, i: number) => {
    const d = e.key === "ArrowRight" ? 1 : e.key === "ArrowLeft" ? -1 : 0;
    if (!d) return;
    const n = (i + d + tabs.length) % tabs.length;
    setActive(tabs[n].id);
    refs.current[n]?.focus();
  };
  return (
    <>
      {before}
      <div className={className} role="tablist" aria-label={label}>
        {tabs.map((t, i) => (
          <button
            key={t.id}
            ref={(el) => {
              refs.current[i] = el;
            }}
            type="button"
            role="tab"
            id={`tab-${t.id}`}
            aria-controls={`panel-${t.id}`}
            aria-selected={t.id === active}
            tabIndex={t.id === active ? 0 : -1}
            onClick={() => setActive(t.id)}
            onKeyDown={(e) => onKey(e, i)}
          >
            {t.title}
          </button>
        ))}
      </div>
      {tabs.map((t) => (
        <div key={t.id} id={`panel-${t.id}`} role="tabpanel" aria-labelledby={`tab-${t.id}`} hidden={t.id !== active}>
          {t.panel}
        </div>
      ))}
    </>
  );
}

/** The editor's slash menu: the highlight walks through the blocks. */
export function SlashMenu({ items }: { items: { name: string; hint: string }[] }) {
  const still = useStill();
  const [on, setOn] = useState(1);
  useEffect(() => {
    if (still) return;
    const id = setInterval(() => setOn((i) => (i + 1) % items.length), 1300);
    return () => clearInterval(id);
  }, [still, items.length]);
  return (
    <ul>
      {items.map((it, i) => (
        <li key={it.name} className={i === on ? "on" : undefined}>
          {it.name} <span>{it.hint}</span>
        </li>
      ))}
    </ul>
  );
}

/** The tagline lights up word by word as it scrolls into the middle of the screen. */
export function Tagline({ text }: { text: string }) {
  const ref = useRef<HTMLParagraphElement>(null);
  const still = useStill();
  useEffect(() => {
    const p = ref.current;
    if (!p) return;
    const words = [...p.querySelectorAll<HTMLElement>(".w")];
    const light = () => {
      const r = p.getBoundingClientRect();
      const k = still ? 1 : Math.min(1, Math.max(0, (innerHeight * 0.85 - r.top) / (r.height + innerHeight * 0.35)));
      const n = Math.round(k * words.length);
      words.forEach((w, i) => w.classList.toggle("lit", i < n));
    };
    light();
    addEventListener("scroll", light, { passive: true });
    return () => removeEventListener("scroll", light);
  }, [still]);
  const words = text.split(" ");
  return (
    <p ref={ref}>
      {words.map((w, i) => (
        <span key={i}>
          <span className="w">{w}</span>
          {i < words.length - 1 ? " " : null}
        </span>
      ))}
    </p>
  );
}

const SIZES = [100, 200, 400, 800, 1600, 3200, 6400, 10000];
const LIST = 25000;

/** No. 01: a 25,000-reader send in growing waves. */
export function WavePlan() {
  const still = useStill();
  const [state, setState] = useState<("" | "on" | "done")[]>(SIZES.map(() => ""));
  const [status, setStatus] = useState(`8 waves planned for ${fmt(LIST)} readers.`);
  const [running, setRunning] = useState(false);
  const [ran, setRan] = useState(false);
  const alive = useRef(true);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);

  async function run() {
    setRunning(true);
    const s: ("" | "on" | "done")[] = SIZES.map(() => "");
    setState([...s]);
    let total = 0;
    let waves = 0;
    for (let i = 0; i < SIZES.length && total < LIST; i++) {
      const n = Math.min(SIZES[i], LIST - total);
      s[i] = "on";
      setState([...s]);
      setStatus(`Wave ${i + 1}: ${fmt(n)} readers. ${i === 0 ? "Your most engaged readers go first." : "Next wave waits 10 minutes."}`);
      await new Promise((r) => setTimeout(r, still ? 150 : 650));
      if (!alive.current) return;
      s[i] = "done";
      setState([...s]);
      total += n;
      waves++;
    }
    setStatus(`Done. ${fmt(total)} readers in ${waves} waves, about 70 minutes.`);
    setRunning(false);
    setRan(true);
  }

  return (
    <>
      <div className="body">
        <div className="waves" aria-hidden="true">
          {SIZES.map((s, i) => (
            <div key={s} className={`b ${state[i]}`} style={{ height: `${Math.max(4, (s / 10000) * 100)}%` }} />
          ))}
        </div>
        <div className="wlabels" aria-hidden="true">
          {SIZES.map((s) => (
            <span key={s}>{s >= 1000 ? `${s / 1000}k` : s}</span>
          ))}
        </div>
      </div>
      <div className="sendrow">
        <span className="status" aria-live="polite">{status}</span>
        <button className="btn pri" type="button" disabled={running} onClick={run}>
          {ran ? "Run it again" : "Run a send"}
        </button>
      </div>
    </>
  );
}

/** No. 02: drag the bounce rate past 2% and the account pauses. */
export function BounceDemo() {
  const [raw, setRaw] = useState(6);
  const v = raw / 10;
  const over = v > 2;
  return (
    <div className="body gauges">
      <div>
        <div className="gline">
          <span className="eyebrow">Bounce rate</span>
          <span className="px">{v.toFixed(1)}%</span>
        </div>
        <div className="meter" style={{ marginTop: 12 }} aria-hidden="true">
          <div className="fill" style={{ width: `${(v / 4) * 100}%` }} />
          <div className="line" style={{ left: "50%" }} />
        </div>
        <div className="scale" aria-hidden="true">
          <span>0%</span>
          <span>2% pause line</span>
          <span>4%</span>
        </div>
        <label className="eyebrow" htmlFor="b-range" style={{ marginTop: 20 }}>
          Try a bad list
        </label>
        <input
          className="range"
          id="b-range"
          type="range"
          min={0}
          max={40}
          value={raw}
          aria-describedby="b-state"
          onChange={(e) => setRaw(Number(e.target.value))}
        />
      </div>
      <div className="gline">
        <span id="b-state" aria-live="polite">
          {over ? `Bounces hit ${v.toFixed(1)}%. Mailrust paused the account. The rest of your list is safe.` : "Sending normally. Wave 3 goes out at 14:20."}
        </span>
        <span className={over ? "stamp paused" : "stamp"}>{over ? "Auto-paused" : "Sending"}</span>
      </div>
    </div>
  );
}

const CHECKS = [
  { label: "From address is on a verified domain", tag: "DKIM", why: "The from address has to be on a verified domain." },
  { label: "Subject line is filled in", tag: "required", why: "The newsletter needs a subject." },
  { label: "Postal address is set", tag: "CAN-SPAM", why: "Add your postal address in settings. The law in many countries asks for it." },
  { label: "Under Gmail's 100 KB clip", tag: "34 KB", why: "Gmail cuts off emails over 100 KB. Shorten it or use fewer images." },
  { label: "At least one reader to send to", tag: "4,812", why: "Nobody is subscribed yet, or the chosen tags have no readers." },
  { label: "Every block is valid", tag: "18 blocks", why: "One of the blocks is broken, such as a button with no link." },
];

/** No. 04: the Send button stays locked until every check passes. */
export function SendChecks() {
  const [ok, setOk] = useState(CHECKS.map(() => true));
  const [sent, setSent] = useState(false);
  const bad = CHECKS.find((_, i) => !ok[i]);
  return (
    <>
      <ul className="checks">
        {CHECKS.map((c, i) => (
          <li key={c.label}>
            <label>
              <input
                type="checkbox"
                checked={ok[i]}
                onChange={(e) => {
                  setSent(false);
                  setOk((o) => o.map((x, j) => (j === i ? e.target.checked : x)));
                }}
              />
              <span>{c.label}</span>
              <small>{c.tag}</small>
            </label>
          </li>
        ))}
      </ul>
      <div className="sendrow">
        <span className="status" aria-live="polite">
          {bad ? bad.why : sent ? "In the real app, wave 1 would start now." : "All checks pass."}
        </span>
        <button className="btn pri" type="button" disabled={!!bad} onClick={() => setSent(true)}>
          Send newsletter
        </button>
      </div>
    </>
  );
}
