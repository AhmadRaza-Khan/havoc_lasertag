CREATE TABLE players (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    full_name     TEXT NOT NULL,
    playername    TEXT NOT NULL UNIQUE,
    phone         TEXT,
    email         TEXT,
    password_hash TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX players_created_at_idx ON players (created_at DESC);
