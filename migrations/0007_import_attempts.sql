-- Imports: count claims so a job that keeps failing is given up instead of retried forever.
ALTER TABLE contact_imports ADD COLUMN attempts integer NOT NULL DEFAULT 0;
