-- Expo push tokens of the phones players are signed in on. A token belongs to one player at a
-- time: signing in as someone else on the same phone moves it.
CREATE TABLE push_tokens (
    token      TEXT PRIMARY KEY,
    player_id  UUID NOT NULL REFERENCES players (id) ON DELETE CASCADE,
    -- 'ios' or 'android'
    platform   TEXT NOT NULL CHECK (platform IN ('ios', 'android')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX push_tokens_player_idx ON push_tokens (player_id);
