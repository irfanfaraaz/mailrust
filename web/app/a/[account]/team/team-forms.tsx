"use client";
import { useActionState, useEffect, type ReactNode } from "react";
import { toast } from "sonner";
import { NativeSelect } from "@/components/native-select";
import { Button, buttonVariants } from "@/components/ui/button";
import { Dialog, DialogClose, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Stamp } from "@/components/status-badge";
import { TableCell, TableRow } from "@/components/ui/table";
import { cancelInvite, invite, leave, removeMember, setRole, type TeamState } from "./actions";

const none: TeamState = {};

/** Toast once per successful submit: each action result is a new object, the initial state is `none`. */
function useSuccessToast(state: TeamState, message: string) {
  useEffect(() => {
    if (state !== none && !state.error) toast.success(message);
  }, [state, message]);
}

function Alert({ state }: { state: TeamState }) {
  return state.error ? <span role="alert" className="text-sm text-destructive">{state.error}</span> : null;
}

export function InviteForm({ account }: { account: string }) {
  const [state, action, pending] = useActionState(invite.bind(null, account), none);
  useSuccessToast(state, "Invite sent");
  return (
    <Card>
      <CardHeader>
        <h2 className="text-lg leading-snug font-semibold">Invite someone</h2>
      </CardHeader>
      <CardContent>
        <form action={action} className="flex flex-wrap items-end gap-3 text-sm">
          <label className="flex flex-col gap-1.5"><span className="label-caps text-muted-foreground">Email</span>
            <Input name="email" type="email" required />
          </label>
          <label className="flex flex-col gap-1.5"><span className="label-caps text-muted-foreground">Invite as</span>
            <NativeSelect name="role" defaultValue="member">
              <option value="member">member</option>
              <option value="owner">owner</option>
            </NativeSelect>
          </label>
          <Button type="submit" disabled={pending}>Send invite</Button>
          <Alert state={state} />
        </form>
      </CardContent>
    </Card>
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

export function MemberRow(p: { account: string; userId: string; email: string; role: string; joined: ReactNode; isOwner: boolean; isMe: boolean }) {
  const [roleState, roleAction] = useActionState(setRole.bind(null, p.account, p.userId), none);
  // The row unmounts once the member is gone, so toast from the action itself rather than an effect.
  const [removeState, removeAction] = useActionState(async (prev: TeamState) => {
    const r = await removeMember(p.account, p.userId, prev);
    if (!r.error) toast.success("Member removed");
    return r;
  }, none);
  const [leaveState, leaveAction] = useActionState(leave.bind(null, p.account, p.userId), none);
  useSuccessToast(roleState, "Role saved");
  return (
    <TableRow>
      <TableCell className="font-medium">{p.email}{p.isMe && " (you)"}</TableCell>
      <TableCell>
        {p.isOwner ? (
          <form action={roleAction} className="flex items-center gap-2">
            <NativeSelect name="role" aria-label="Role" defaultValue={p.role}>
              <option value="member">member</option>
              <option value="owner">owner</option>
            </NativeSelect>
            <Button type="submit" size="sm" variant="outline">Save role</Button>
          </form>
        ) : <Stamp tone="quiet">{p.role}</Stamp>}
      </TableCell>
      <TableCell className="tabular-nums text-muted-foreground">{p.joined}</TableCell>
      <TableCell>
        <div className="flex flex-wrap items-center justify-end gap-2">
          <Alert state={roleState.error ? roleState : removeState.error ? removeState : leaveState} />
          {p.isOwner && !p.isMe && (
            <Confirm trigger="Remove" title="Remove this member?" description={`${p.email} loses access to this account.`} confirm="Remove member" action={removeAction as () => void} />
          )}
          {p.isMe && (
            <Confirm trigger="Leave account" title="Leave this account?" description="You lose access until someone invites you again." confirm="Leave" action={leaveAction as () => void} />
          )}
        </div>
      </TableCell>
    </TableRow>
  );
}

export function InviteRow({ account, id, email, role, expires }: { account: string; id: string; email: string; role: string; expires: ReactNode }) {
  const [state, action] = useActionState(cancelInvite.bind(null, account, id), none);
  return (
    <TableRow>
      <TableCell className="font-medium">{email}</TableCell>
      <TableCell><Stamp tone="quiet">{role}</Stamp></TableCell>
      <TableCell className="tabular-nums text-muted-foreground">Pending, expires {expires}</TableCell>
      <TableCell>
        <div className="flex items-center justify-end gap-2">
          <Alert state={state} />
          <form action={action}>
            <Button type="submit" size="sm" variant="outline">Cancel invite</Button>
          </form>
        </div>
      </TableCell>
    </TableRow>
  );
}
