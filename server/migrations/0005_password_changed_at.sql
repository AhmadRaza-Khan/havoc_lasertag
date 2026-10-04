-- Tokens issued before this time are rejected, so resetting a password signs out every
-- existing session. Existing rows get the epoch so current sessions stay valid.
ALTER TABLE users ADD COLUMN password_changed_at TIMESTAMPTZ NOT NULL DEFAULT 'epoch';
ALTER TABLE users ALTER COLUMN password_changed_at SET DEFAULT date_trunc('second', now());

ALTER TABLE players ADD COLUMN password_changed_at TIMESTAMPTZ NOT NULL DEFAULT 'epoch';
ALTER TABLE players ALTER COLUMN password_changed_at SET DEFAULT date_trunc('second', now());
