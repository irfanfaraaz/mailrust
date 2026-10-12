-- 0003 added this for the urgent cap, but the cap query never uses it. It only slows inserts.
DROP INDEX IF EXISTS emails_account_urgent;
