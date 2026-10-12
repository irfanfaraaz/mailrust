CREATE TYPE account_status AS ENUM ('active', 'paused');
CREATE TYPE key_access AS ENUM ('full', 'sending');
CREATE TYPE domain_status AS ENUM ('pending', 'verified', 'failed');
CREATE TYPE email_status AS ENUM ('queued', 'sending', 'unknown', 'sent', 'delivered', 'delivery_delayed', 'bounced', 'complained', 'failed', 'suppressed', 'canceled');
CREATE TYPE suppression_reason AS ENUM ('bounce', 'complaint', 'manual');
CREATE TYPE delivery_status AS ENUM ('pending', 'delivered', 'failed');

CREATE TABLE accounts (
    id uuid PRIMARY KEY,
    name text NOT NULL,
    status account_status NOT NULL DEFAULT 'active',
    paused_reason text,
    paused_at timestamptz,
    ses_tenant_name text NOT NULL UNIQUE,
    ses_synced_at timestamptz,
    send_rate_per_sec integer NOT NULL DEFAULT 10 CHECK (send_rate_per_sec > 0),
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE api_keys (
    id uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    name text NOT NULL,
    prefix text NOT NULL,
    key_hash bytea NOT NULL UNIQUE,
    access key_access NOT NULL,
    last_used_at timestamptz,
    revoked_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX api_keys_account ON api_keys (account_id);

CREATE TABLE domains (
    id uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    name text NOT NULL,
    status domain_status NOT NULL DEFAULT 'pending',
    dns_records jsonb NOT NULL DEFAULT '[]',
    tracking_subdomain text,
    tracking_verified boolean NOT NULL DEFAULT false,
    open_tracking boolean NOT NULL DEFAULT false,
    click_tracking boolean NOT NULL DEFAULT false,
    verify_until timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (account_id, name)
);
CREATE UNIQUE INDEX domains_tracking_subdomain ON domains (tracking_subdomain) WHERE tracking_subdomain IS NOT NULL;

CREATE TABLE emails (
    id uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    domain_id uuid REFERENCES domains(id) ON DELETE SET NULL,
    from_address text NOT NULL,
    to_addresses text[] NOT NULL,
    cc_addresses text[] NOT NULL DEFAULT '{}',
    bcc_addresses text[] NOT NULL DEFAULT '{}',
    reply_to text[] NOT NULL DEFAULT '{}',
    subject text NOT NULL,
    html text,
    text_body text,
    headers jsonb NOT NULL DEFAULT '{}',
    tags jsonb NOT NULL DEFAULT '{}',
    links jsonb NOT NULL DEFAULT '[]',
    status email_status NOT NULL DEFAULT 'queued',
    send_at timestamptz NOT NULL DEFAULT now(),
    sent_at timestamptz,
    attempts integer NOT NULL DEFAULT 0,
    lease_until timestamptz,
    unknown_since timestamptz,
    provider_message_id text,
    idempotency_key text,
    request_hash bytea,
    last_error text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (html IS NOT NULL OR text_body IS NOT NULL)
);
CREATE INDEX emails_due ON emails (send_at) WHERE status = 'queued';
CREATE INDEX emails_account_id ON emails (account_id, id DESC);
CREATE UNIQUE INDEX emails_idempotency ON emails (account_id, idempotency_key) WHERE idempotency_key IS NOT NULL;
CREATE INDEX emails_provider_message ON emails (provider_message_id) WHERE provider_message_id IS NOT NULL;
CREATE INDEX emails_sending_lease ON emails (lease_until) WHERE status = 'sending';
CREATE INDEX emails_unknown ON emails (unknown_since) WHERE status = 'unknown';

CREATE TABLE attachments (
    id uuid PRIMARY KEY,
    email_id uuid NOT NULL REFERENCES emails(id) ON DELETE CASCADE,
    account_id uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    filename text NOT NULL,
    content_type text NOT NULL,
    content bytea NOT NULL
);
CREATE INDEX attachments_email ON attachments (email_id);

CREATE TABLE email_events (
    id uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    email_id uuid NOT NULL REFERENCES emails(id) ON DELETE CASCADE,
    type text NOT NULL,
    recipient text,
    data jsonb NOT NULL DEFAULT '{}',
    likely_bot boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX email_events_email ON email_events (email_id);
CREATE INDEX email_events_account_type_time ON email_events (account_id, type, created_at);

-- SNS can deliver a message twice; one row per SNS MessageId makes the second a no-op.
CREATE TABLE sns_messages (
    message_id text PRIMARY KEY,
    received_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE suppressions (
    account_id uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    address text NOT NULL,
    reason suppression_reason NOT NULL,
    source_email_id uuid,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (account_id, address)
);

CREATE TABLE webhooks (
    id uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    url text NOT NULL,
    secret text NOT NULL,
    event_types text[] NOT NULL,
    enabled boolean NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX webhooks_account ON webhooks (account_id);

CREATE TABLE webhook_deliveries (
    id uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    webhook_id uuid NOT NULL REFERENCES webhooks(id) ON DELETE CASCADE,
    event_type text NOT NULL,
    payload jsonb NOT NULL,
    status delivery_status NOT NULL DEFAULT 'pending',
    attempts integer NOT NULL DEFAULT 0,
    next_attempt_at timestamptz NOT NULL DEFAULT now(),
    lease_until timestamptz,
    last_response_code integer,
    last_error text,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX webhook_deliveries_due ON webhook_deliveries (next_attempt_at) WHERE status = 'pending';
