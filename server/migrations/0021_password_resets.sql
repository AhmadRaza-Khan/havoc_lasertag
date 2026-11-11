-- "Forgot password" for players: a 6-digit code emailed to them. One pending code per player;
-- asking again replaces it. The code is stored hashed, expires, and allows a few tries only.
CREATE TABLE player_password_resets (
    player_id  UUID PRIMARY KEY REFERENCES players (id) ON DELETE CASCADE,
    code_hash  TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    attempts   INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
