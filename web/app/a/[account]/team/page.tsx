import { Suspense } from "react";
import { engine, type Schemas } from "@/lib/engine";
import { currentAccount } from "@/lib/account";
import { InviteForm, InviteRow, MemberRow } from "./team-forms";

type Props = { params: Promise<{ account: string }> };

export default function Team(props: Props) {
  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-2xl font-semibold">Team</h1>
      {/* params and the engine calls are request data: read them inside Suspense (cacheComponents). */}
      <Suspense>
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
  const date = (s: string) => new Date(s).toLocaleDateString();
  return (
    <>
      {isOwner && <InviteForm account={account} />}
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead><tr><th className="text-left">Email</th><th className="text-left">Role</th><th className="text-left">Joined</th><th /></tr></thead>
          <tbody>
            {team.members.map((m) => (
              <MemberRow key={m.user_id} account={account} userId={m.user_id} email={m.email} role={m.role} joined={date(m.joined_at)} isOwner={isOwner} isMe={m.user_id === me.user.id} />
            ))}
            {isOwner && team.invites.map((i) => (
              <InviteRow key={i.id} account={account} id={i.id} email={i.email} role={i.role} expires={date(i.expires_at)} />
            ))}
          </tbody>
        </table>
      </div>
    </>
  );
}
