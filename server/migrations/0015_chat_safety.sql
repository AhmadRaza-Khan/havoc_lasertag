-- Chat safety (App Store guideline 1.2, Google Play UGC policy): players can block each other
-- and report players or messages; staff review reports in the admin panel.

CREATE TABLE player_blocks (
    blocker_id UUID NOT NULL REFERENCES players (id) ON DELETE CASCADE,
    blocked_id UUID NOT NULL REFERENCES players (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (blocker_id, blocked_id),
    CHECK (blocker_id <> blocked_id)
);

CREATE INDEX player_blocks_blocked_idx ON player_blocks (blocked_id);

CREATE TABLE chat_reports (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    reporter_id         UUID REFERENCES players (id) ON DELETE SET NULL,
    reported_id         UUID REFERENCES players (id) ON DELETE SET NULL,
    -- Kept as text so the report still reads right if an account is deleted
    reporter_playername TEXT NOT NULL,
    reported_playername TEXT NOT NULL,
    -- The reported message, if one was (a copy: the message may be deleted later)
    message_id          UUID,
    message_body        TEXT,
    reason              TEXT NOT NULL CHECK (reason IN ('spam', 'harassment', 'inappropriate', 'other')),
    details             TEXT CHECK (char_length(details) <= 1000),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    resolved_at         TIMESTAMPTZ
);

CREATE INDEX chat_reports_open_idx ON chat_reports (created_at DESC) WHERE resolved_at IS NULL;
