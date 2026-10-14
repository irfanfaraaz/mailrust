"use client";
import { Menu } from "@base-ui/react/menu";
import { ChevronsUpDownIcon } from "lucide-react";
import Link from "next/link";
import { logout, logoutAll } from "@/app/login/actions";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import type { Schemas } from "@/lib/engine";

const item = "flex cursor-default items-center rounded-md px-1.5 py-1 text-sm outline-hidden select-none focus:bg-accent focus:text-accent-foreground";
const trigger = "flex w-full items-center justify-between gap-2 rounded-md px-2 py-1.5 text-left text-sm hover:bg-sidebar-accent max-md:min-h-11";

export function AccountMenu({ me, current }: { me: Schemas["MeResponse"]; current: Schemas["MyAccount"] }) {
  return (
    <div className="flex flex-col gap-0.5 p-3 pb-2">
      <DropdownMenu>
        <DropdownMenuTrigger aria-label="Switch account" className={trigger}>
          <span className="flex min-w-0 items-center gap-2.5">
            <span aria-hidden className="grid size-8 shrink-0 place-items-center rounded-md bg-primary font-heading text-base font-semibold text-primary-foreground">
              {current.name.charAt(0).toUpperCase()}
            </span>
            <span className="flex min-w-0 flex-col">
              <span className="truncate font-semibold">{current.name}</span>
              <span className="truncate text-xs text-muted-foreground capitalize">{current.role}</span>
            </span>
          </span>
          <ChevronsUpDownIcon aria-hidden className="size-4 shrink-0 text-muted-foreground" />
        </DropdownMenuTrigger>
        <DropdownMenuContent>
          {me.accounts.map((a) => (
            <Menu.LinkItem key={a.id} closeOnClick className={item} render={<Link href={`/a/${a.id}/overview`} />}>
              {a.name} · {a.role}
            </Menu.LinkItem>
          ))}
        </DropdownMenuContent>
      </DropdownMenu>
      <DropdownMenu>
        <DropdownMenuTrigger aria-label="Account menu" className={`${trigger} text-xs text-muted-foreground`}>
          <span className="truncate">{me.user.email}</span>
        </DropdownMenuTrigger>
        <DropdownMenuContent>
          <form action={logout}>
            <DropdownMenuItem nativeButton render={<button type="submit" className="w-full" />}>Log out</DropdownMenuItem>
          </form>
          <DropdownMenuSeparator />
          <form action={logoutAll}>
            <DropdownMenuItem nativeButton render={<button type="submit" className="w-full" />}>Log out everywhere</DropdownMenuItem>
          </form>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}
