-- Accounts (QA follow-up): staff emails and an owner, player email verification, one table for
-- every emailed code, and refund tracking for paid bookings that get canceled.

-- Staff: an email to reset their own password, and an owner who manages staff accounts.
ALTER TABLE users ADD COLUMN email TEXT;
CREATE UNIQUE INDEX users_email_unique ON users (lower(email)) WHERE email IS NOT NULL;
ALTER TABLE users ADD COLUMN is_owner BOOLEAN NOT NULL DEFAULT false;
-- The first account created is the owner.
UPDATE users SET is_owner = true
WHERE id = (SELECT id FROM users ORDER BY created_at, id LIMIT 1);

-- Players: set when they enter the code emailed to them; cleared when the email changes.
ALTER TABLE players ADD COLUMN email_verified_at TIMESTAMPTZ;

-- 6-digit codes sent by email, one pending code per account and purpose (asking again
-- replaces it). Stored hashed; they expire and allow a few tries only. `subject_id` is a
-- player or a staff user, depending on the purpose.
CREATE TABLE email_codes (
    subject_id UUID NOT NULL,
    purpose    TEXT NOT NULL CHECK (purpose IN ('player_reset', 'player_verify', 'player_delete', 'staff_reset')),
    code_hash  TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    attempts   INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (subject_id, purpose)
);
-- Replaced by email_codes (purpose 'player_reset').
DROP TABLE player_password_resets;

-- Paid bookings that are canceled: "pending" until staff mark the money refunded (or SkipCash
-- reports the refund).
ALTER TABLE bookings ADD COLUMN refund_status TEXT CHECK (refund_status IN ('pending', 'refunded'));
ALTER TABLE bookings ADD COLUMN refunded_at TIMESTAMPTZ;
UPDATE bookings SET refund_status = 'pending' WHERE status = 'canceled' AND amount_paid > 0;
