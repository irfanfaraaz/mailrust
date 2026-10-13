import { Suspense } from "react";
import { Loading } from "@/components/loading";
import { Table, TableBody, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { engine, type Schemas } from "@/lib/engine";
import { currentAccount } from "@/lib/account";
import { utc } from "@/lib/format";
import { InviteForm, InviteRow, MemberRow } from "./team-forms";

export const metadata = { title: "Team" };

type Props = { params: Promise<{ account: string }> };

export default function Team(props: Props) {
  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-2xl font-semibold">Team</h1>
      {/* params and the engine calls are request data: read them inside Suspense (cacheComponents). */}
      <Suspense fallback={<Loading />}>
        <Body {...props} />
      </Suspense>
    </div>
  );
}

async function Body({ params }: Props) {
  const { account } = await params;
  const nextPath = `/a/${account}/team`;
  const [team, { me, account: mine }] = await Promise.all([
    engine<Schemas["TeamView"]>("/team", { account, nextPath }),
    currentAccount(account, nextPath),
  ]);
  const isOwner = mine.role === "owner";
  return (
    <>
      {isOwner && <InviteForm account={account} />}
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Email</TableHead>
            <TableHead>Role</TableHead>
            <TableHead>Joined</TableHead>
            <TableHead><span className="sr-only">Actions</span></TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {team.members.map((m) => (
            <MemberRow key={m.user_id} account={account} userId={m.user_id} email={m.email} role={m.role} joined={utc(m.joined_at)} isOwner={isOwner} isMe={m.user_id === me.user.id} />
          ))}
          {isOwner && team.invites.map((i) => (
            <InviteRow key={i.id} account={account} id={i.id} email={i.email} role={i.role} expires={utc(i.expires_at)} />
          ))}
        </TableBody>
      </Table>
    </>
  );
}
