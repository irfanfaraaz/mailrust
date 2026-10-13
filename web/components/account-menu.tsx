"use client";
import { Menu } from "@base-ui/react/menu";
import Link from "next/link";
import { logout, logoutAll } from "@/app/login/actions";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import type { Schemas } from "@/lib/engine";

const item = "flex cursor-default items-center rounded-md px-1.5 py-1 text-sm outline-hidden select-none focus:bg-accent focus:text-accent-foreground";
const trigger = "flex w-full items-center justify-between gap-2 rounded px-2 py-1.5 text-left text-sm hover:bg-muted max-md:min-h-11";

export function AccountMenu({ me, current }: { me: Schemas["MeResponse"]; current: Schemas["MyAccount"] }) {
  return (
    <div className="flex flex-col gap-1 border-b p-3 max-md:border-b-0">
      <DropdownMenu>
        <DropdownMenuTrigger aria-label="Switch account" className={`${trigger} font-medium`}>
          <span className="truncate">{current.name}</span>
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
        <DropdownMenuTrigger aria-label="Account menu" className={`${trigger} text-muted-foreground`}>
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
