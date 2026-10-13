-- When a contact last became pending (a join that sent a confirm email). The 30-day cleanup keys on it;
-- updated_at is bumped by imports and edits, so it cannot tell how long a signup went unconfirmed.
ALTER TABLE contacts ADD COLUMN pending_since timestamptz;
UPDATE contacts SET pending_since = updated_at WHERE status = 'pending';
