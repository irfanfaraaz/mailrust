"use client";
import { useActionState } from "react";
import { Button, buttonVariants } from "@/components/ui/button";
import { Dialog, DialogClose, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { cancelInvite, invite, leave, removeMember, setRole, type TeamState } from "./actions";

const SELECT = "h-8 rounded-md border bg-background px-2 text-sm";
const none: TeamState = {};

function Alert({ state }: { state: TeamState }) {
  return state.error ? <span role="alert" className="text-sm text-destructive">{state.error}</span> : null;
}

export function InviteForm({ account }: { account: string }) {
  const [state, action, pending] = useActionState(invite.bind(null, account), none);
  return (
    <form action={action} className="flex flex-wrap items-end gap-3 text-sm">
      <label className="flex flex-col gap-1">Email
        <Input name="email" type="email" required />
      </label>
      <label className="flex flex-col gap-1">Invite as
        <select name="role" defaultValue="member" className={SELECT}>
          <option value="member">member</option>
          <option value="owner">owner</option>
        </select>
      </label>
      <Button type="submit" disabled={pending}>Send invite</Button>
      <Alert state={state} />
    </form>
  );
}

function Confirm({ trigger, title, description, confirm, action }: { trigger: string; title: string; description: string; confirm: string; action: () => void }) {
  return (
    <Dialog>
      <DialogTrigger className={buttonVariants({ variant: "outline", size: "sm" })}>{trigger}</DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription>{description}</DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <DialogClose className={buttonVariants({ variant: "outline" })}>Cancel</DialogClose>
          <form action={action}>
            <button type="submit" className={buttonVariants({ variant: "destructive" })}>{confirm}</button>
          </form>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export function MemberRow(p: { account: string; userId: string; email: string; role: string; joined: string; isOwner: boolean; isMe: boolean }) {
  const [roleState, roleAction] = useActionState(setRole.bind(null, p.account, p.userId), none);
  const [removeState, removeAction] = useActionState(removeMember.bind(null, p.account, p.userId), none);
  const [leaveState, leaveAction] = useActionState(leave.bind(null, p.account, p.userId), none);
  return (
    <tr>
      <td>{p.email}{p.isMe && " (you)"}</td>
      <td>
        {p.isOwner ? (
          <form action={roleAction} className="flex items-center gap-2">
            <select name="role" aria-label="Role" defaultValue={p.role} className={SELECT}>
              <option value="member">member</option>
              <option value="owner">owner</option>
            </select>
            <Button type="submit" size="sm" variant="outline">Save role</Button>
          </form>
        ) : p.role}
      </td>
      <td className="tabular-nums">{p.joined}</td>
      <td className="flex flex-wrap items-center justify-end gap-2">
        <Alert state={roleState.error ? roleState : removeState.error ? removeState : leaveState} />
        {p.isOwner && !p.isMe && (
          <Confirm trigger="Remove" title="Remove this member?" description={`${p.email} loses access to this account.`} confirm="Remove member" action={removeAction as () => void} />
        )}
        {p.isMe && (
          <Confirm trigger="Leave account" title="Leave this account?" description="You lose access until someone invites you again." confirm="Leave" action={leaveAction as () => void} />
        )}
      </td>
    </tr>
  );
}

export function InviteRow({ account, id, email, role, expires }: { account: string; id: string; email: string; role: string; expires: string }) {
  const [state, action] = useActionState(cancelInvite.bind(null, account, id), none);
  return (
    <tr>
      <td>{email}</td>
      <td>{role}</td>
      <td className="tabular-nums">Pending, expires {expires}</td>
      <td className="flex items-center justify-end gap-2">
        <Alert state={state} />
        <form action={action}>
          <Button type="submit" size="sm" variant="outline">Cancel invite</Button>
        </form>
      </td>
    </tr>
  );
}
