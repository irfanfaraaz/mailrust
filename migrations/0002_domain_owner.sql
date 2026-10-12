-- SES identities belong to the whole AWS account, so a domain may belong to only one Mailrust account.
-- Otherwise a second account adding a verified domain would be "verified" at once and could send as it.
ALTER TABLE domains DROP CONSTRAINT domains_account_id_name_key;
ALTER TABLE domains ADD CONSTRAINT domains_name_key UNIQUE (name);
