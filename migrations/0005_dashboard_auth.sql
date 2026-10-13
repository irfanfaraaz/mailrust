-- Dashboard login: users, memberships, sessions, magic links, invites.
CREATE TYPE member_role AS ENUM ('member', 'owner');

CREATE TABLE users (
    id            uuid PRIMARY KEY,
    email         text NOT NULL UNIQUE CHECK (email = lower(email)),
    created_at    timestamptz NOT NULL DEFAULT now(),
    last_login_at timestamptz
);

CREATE TABLE account_members (
    account_id uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    user_id    uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role       member_role NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (account_id, user_id)
);
CREATE INDEX account_members_user ON account_members (user_id);

CREATE TABLE sessions (
    id           uuid PRIMARY KEY,
    user_id      uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash   bytea NOT NULL UNIQUE,
    created_at   timestamptz NOT NULL DEFAULT now(),
    last_used_at timestamptz NOT NULL DEFAULT now(),
    expires_at   timestamptz NOT NULL
);
CREATE INDEX sessions_user ON sessions (user_id);

CREATE TABLE login_tokens (
    token_hash bytea PRIMARY KEY,
    user_id    uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL,
    used_at    timestamptz
);

CREATE TABLE invites (
    id          uuid PRIMARY KEY,
    account_id  uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    email       text NOT NULL CHECK (email = lower(email)),
    role        member_role NOT NULL,
    token_hash  bytea NOT NULL UNIQUE,
    invited_by  uuid REFERENCES users(id) ON DELETE SET NULL,
    created_at  timestamptz NOT NULL DEFAULT now(),
    expires_at  timestamptz NOT NULL,
    accepted_at timestamptz,
    revoked_at  timestamptz
);
-- One open invite per address per account; a new invite revokes the old one first.
CREATE UNIQUE INDEX invites_one_open ON invites (account_id, email)
    WHERE accepted_at IS NULL AND revoked_at IS NULL;

CREATE TABLE login_requests (
    email text NOT NULL,
    ip    inet,
    at    timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX login_requests_email ON login_requests (email, at);
CREATE INDEX login_requests_ip ON login_requests (ip, at);
