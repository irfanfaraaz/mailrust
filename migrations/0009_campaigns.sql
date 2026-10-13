-- Campaigns: newsletter issues, their readers (send state), uploaded images, and the postal address.
CREATE TYPE campaign_status AS ENUM ('draft', 'scheduled', 'sending', 'paused', 'sent', 'cancelled');

ALTER TABLE accounts
    ADD COLUMN postal_address  text CHECK (char_length(postal_address) BETWEEN 1 AND 300),
    ADD COLUMN signup_ask_name boolean NOT NULL DEFAULT true;

CREATE TABLE campaigns (
    id             uuid PRIMARY KEY,
    account_id     uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    subject        text NOT NULL DEFAULT '' CHECK (char_length(subject) <= 200),
    preview_text   text CHECK (char_length(preview_text) <= 200),
    blocks         jsonb NOT NULL DEFAULT '{"type":"doc","content":[]}' CHECK (jsonb_typeof(blocks) = 'object'),
    include_tags   uuid[] NOT NULL DEFAULT '{}',
    exclude_tags   uuid[] NOT NULL DEFAULT '{}',
    status         campaign_status NOT NULL DEFAULT 'draft',
    send_at        timestamptz,
    started_at     timestamptz,
    finished_at    timestamptz,
    paused_reason  text,
    -- 60 characters from the subject plus a "-N" suffix on a clash.
    archive_slug   text CHECK (archive_slug ~ '^[a-z0-9-]{1,64}$'),
    archive_hidden boolean NOT NULL DEFAULT false,
    -- The last wave started (0 = none yet).
    wave           integer NOT NULL DEFAULT 0,
    next_wave_at   timestamptz,
    created_by     uuid REFERENCES users(id) ON DELETE SET NULL,
    created_at     timestamptz NOT NULL DEFAULT now(),
    updated_at     timestamptz NOT NULL DEFAULT now(),
    UNIQUE (account_id, archive_slug)
);
CREATE INDEX campaigns_account ON campaigns (account_id, id DESC);
CREATE INDEX campaigns_due ON campaigns (send_at) WHERE status = 'scheduled';
CREATE INDEX campaigns_active ON campaigns (account_id) WHERE status = 'sending';
-- One issue in `sending` per account (spec §6.2): protects reputation and keeps waves simple.
CREATE UNIQUE INDEX campaigns_one_sending ON campaigns (account_id) WHERE status = 'sending';

-- Send state, not history: a deleted contact's row goes with them (GDPR); their emails row stays.
CREATE TABLE campaign_recipients (
    campaign_id uuid NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    contact_id  uuid NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
    tier        smallint NOT NULL CHECK (tier BETWEEN 1 AND 3),
    position    integer NOT NULL,
    wave        integer NOT NULL,
    email_id    uuid,
    PRIMARY KEY (campaign_id, contact_id)
);
CREATE INDEX campaign_recipients_wave ON campaign_recipients (campaign_id, wave);

ALTER TABLE emails
    ADD COLUMN campaign_id uuid REFERENCES campaigns(id) ON DELETE CASCADE,
    ADD COLUMN contact_id  uuid REFERENCES contacts(id) ON DELETE SET NULL;
-- 0001 named its unnamed table CHECK `emails_check`. Campaign rows have no stored body: the worker renders them.
ALTER TABLE emails DROP CONSTRAINT emails_check;
ALTER TABLE emails ADD CONSTRAINT emails_body CHECK (html IS NOT NULL OR text_body IS NOT NULL OR campaign_id IS NOT NULL);
CREATE INDEX emails_campaign ON emails (campaign_id, status) WHERE campaign_id IS NOT NULL;
-- Tier 2 ("no email yet") and ON DELETE SET NULL both look emails up by contact.
CREATE INDEX emails_contact ON emails (contact_id) WHERE contact_id IS NOT NULL;

CREATE TABLE images (
    id           uuid PRIMARY KEY,
    account_id   uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    content_type text NOT NULL CHECK (content_type IN ('image/png', 'image/jpeg', 'image/gif', 'image/webp')),
    bytes        bytea NOT NULL,
    size         integer NOT NULL CHECK (size BETWEEN 1 AND 5242880),
    width        integer NOT NULL CHECK (width > 0),
    height       integer NOT NULL CHECK (height > 0),
    created_at   timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX images_account ON images (account_id, id DESC);
