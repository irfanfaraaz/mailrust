import { TitleScribble } from "./title-scribble";

/** The top of every page: a serif title, one line of context, and the page's actions on the right. */
export function PageHeader({ title, description, actions, children }: { title: React.ReactNode; description?: React.ReactNode; actions?: React.ReactNode; children?: React.ReactNode }) {
  return (
    <header className="flex flex-wrap items-end justify-between gap-x-6 gap-y-4">
      <div className="flex min-w-0 flex-col gap-2.5">
        {children}
        <h1 className="relative w-fit text-3xl leading-tight font-semibold">
          {title}
          <TitleScribble />
        </h1>
        {description && <p className="max-w-prose text-muted-foreground">{description}</p>}
      </div>
      {actions && <div className="flex flex-wrap items-center gap-2.5">{actions}</div>}
    </header>
  );
}
