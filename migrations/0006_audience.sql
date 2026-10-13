-- Audience: contacts, tags, double opt-in, CSV imports, public signup settings.
CREATE TYPE contact_status AS ENUM ('pending', 'subscribed', 'unsubscribed');
CREATE TYPE contact_source AS ENUM ('form', 'import', 'api', 'dashboard');
CREATE TYPE import_status AS ENUM ('queued', 'running', 'done', 'failed');

ALTER TABLE accounts
    ADD COLUMN public_slug     text UNIQUE CHECK (public_slug ~ '^[a-z0-9-]{3,40}$'),
    ADD COLUMN newsletter_name text CHECK (char_length(newsletter_name) BETWEEN 1 AND 100),
    ADD COLUMN newsletter_from text,
    ADD COLUMN signup_enabled  boolean NOT NULL DEFAULT false;

CREATE TABLE contacts (
    id                    uuid PRIMARY KEY,
    account_id            uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    email                 text NOT NULL CHECK (email = lower(email)),
    first_name            text,
    last_name             text,
    fields                jsonb NOT NULL DEFAULT '{}' CHECK (jsonb_typeof(fields) = 'object'),
    status                contact_status NOT NULL,
    source                contact_source NOT NULL,
    -- True when the reader left through their own unsubscribe link: only then may that link resubscribe them.
    unsubscribed_via_link boolean NOT NULL DEFAULT false,
    subscribed_at         timestamptz,
    unsubscribed_at       timestamptz,
    created_at            timestamptz NOT NULL DEFAULT now(),
    updated_at            timestamptz NOT NULL DEFAULT now(),
    UNIQUE (account_id, email)
);
-- Ids are UUIDv7, so id order is creation order: list paging walks this index newest first.
CREATE INDEX contacts_page ON contacts (account_id, id DESC);
CREATE INDEX contacts_status ON contacts (account_id, status, id DESC);
CREATE INDEX contacts_pending ON contacts (updated_at) WHERE status = 'pending';

CREATE TABLE tags (
    id         uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    name       text NOT NULL CHECK (char_length(name) BETWEEN 1 AND 50 AND name = btrim(name)),
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX tags_name ON tags (account_id, lower(name));

CREATE TABLE contact_tags (
    contact_id uuid NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
    tag_id     uuid NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (contact_id, tag_id)
);
CREATE INDEX contact_tags_tag ON contact_tags (tag_id);

CREATE TABLE contact_confirmations (
    token_hash bytea PRIMARY KEY,
    contact_id uuid NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL,
    used_at    timestamptz
);
CREATE INDEX contact_confirmations_contact ON contact_confirmations (contact_id, created_at);

CREATE TABLE contact_imports (
    id          uuid PRIMARY KEY,
    account_id  uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    created_by  uuid REFERENCES users(id) ON DELETE SET NULL,
    status      import_status NOT NULL DEFAULT 'queued',
    file        bytea,
    tag_id      uuid REFERENCES tags(id) ON DELETE SET NULL,
    rows_total  integer NOT NULL,
    rows_done   integer NOT NULL DEFAULT 0,
    added       integer NOT NULL DEFAULT 0,
    updated     integer NOT NULL DEFAULT 0,
    skipped     integer NOT NULL DEFAULT 0,
    errors      jsonb NOT NULL DEFAULT '[]',
    error       text,
    lease_until timestamptz,
    created_at  timestamptz NOT NULL DEFAULT now(),
    finished_at timestamptz
);
CREATE INDEX contact_imports_todo ON contact_imports (created_at) WHERE status IN ('queued', 'running');
CREATE INDEX contact_imports_account ON contact_imports (account_id, id DESC);

CREATE TABLE join_requests (
    ip         inet,
    email      text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX join_requests_ip ON join_requests (ip, created_at);
