-- Declaration order is the claim order: urgent first.
CREATE TYPE email_priority AS ENUM ('urgent', 'normal', 'bulk');
ALTER TABLE emails ADD COLUMN priority email_priority NOT NULL DEFAULT 'normal';

DROP INDEX emails_due;
CREATE INDEX emails_claim ON emails (priority, send_at) WHERE status = 'queued';
-- Urgent cap counts per account over the last 24 h.
CREATE INDEX emails_account_created ON emails (account_id, created_at);
CREATE INDEX emails_account_urgent ON emails (account_id, created_at) WHERE priority = 'urgent';
-- Reserve estimate: recipients sent since the last quota sync.
CREATE INDEX emails_sent_at ON emails (sent_at) WHERE sent_at IS NOT NULL;

-- One row: the sender's live limits (SES GetAccount) and the global daily-cap pause.
CREATE TABLE sender_quota (
    id boolean PRIMARY KEY DEFAULT true CHECK (id),
    max_send_rate double precision,
    max_24h_send double precision,
    sent_last_24h double precision,
    synced_at timestamptz,
    paused_until timestamptz,
    pause_reason text
);
