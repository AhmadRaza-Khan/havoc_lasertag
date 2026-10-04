-- Game statistics imported from the arena's "Total_statistic_HH:MM.pdf" reports.
-- Append-only: imports add games, they never modify or delete earlier ones.

CREATE TABLE games (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    started_at      TIMESTAMPTZ NOT NULL,
    ended_at        TIMESTAMPTZ NOT NULL,
    scenario        TEXT,
    -- Same game exported twice (even as a byte-different file) has the same fingerprint.
    fingerprint     TEXT NOT NULL UNIQUE,
    file_sha256     TEXT NOT NULL UNIQUE,
    source_filename TEXT NOT NULL,
    imported_by     UUID REFERENCES users (id) ON DELETE SET NULL,
    -- Differences/unreadable parts found while importing (for the admin)
    warnings        TEXT[] NOT NULL DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX games_started_at_idx ON games (started_at DESC);

-- One row per player per game. `player_id` is NULL when the name in the file matched no
-- player (the row is kept, so nothing from the file is lost) or the player was deleted later.
CREATE TABLE game_players (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    game_id          UUID NOT NULL REFERENCES games (id) ON DELETE CASCADE,
    player_id        UUID REFERENCES players (id) ON DELETE SET NULL,
    name_in_file     TEXT NOT NULL,
    number_in_game   INTEGER NOT NULL,

    -- Summary table (page 1): exact text
    points           INTEGER NOT NULL,
    efficiency       DOUBLE PRECISION NOT NULL,
    frags            INTEGER NOT NULL CHECK (frags >= 0),
    deactivated      INTEGER NOT NULL CHECK (deactivated >= 0),
    shots            INTEGER NOT NULL CHECK (shots >= 0),
    hits             INTEGER NOT NULL CHECK (hits >= 0),
    accuracy         INTEGER NOT NULL CHECK (accuracy BETWEEN 0 AND 100),

    -- Personal page (OCR); NULL when that page was missing or a value unreadable
    personal_page    INTEGER,
    team             TEXT,
    team_score       INTEGER,
    efficiency_kdr   DOUBLE PRECISION,
    captures         INTEGER,
    hits_taken       INTEGER,
    max_combo        INTEGER,
    place_in_team    INTEGER,
    place_in_game    INTEGER,
    nominations      TEXT[] NOT NULL DEFAULT '{}',
    victims          JSONB NOT NULL DEFAULT '[]',   -- [{"name": "Zef", "count": 3}]
    insulters        JSONB NOT NULL DEFAULT '[]',

    UNIQUE (game_id, number_in_game),
    UNIQUE (game_id, player_id)
);

CREATE INDEX game_players_player_idx ON game_players (player_id);

-- Each upload and its outcome, so the admin can see what happened to a file.
CREATE TABLE stat_imports (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    status          TEXT NOT NULL CHECK (status IN ('processing', 'completed', 'failed')),
    source_filename TEXT NOT NULL,
    file_sha256     TEXT NOT NULL,
    game_id         UUID REFERENCES games (id) ON DELETE SET NULL,
    matched_players TEXT[] NOT NULL DEFAULT '{}',
    unmatched_names TEXT[] NOT NULL DEFAULT '{}',
    warnings        TEXT[] NOT NULL DEFAULT '{}',
    error           TEXT,
    imported_by     UUID REFERENCES users (id) ON DELETE SET NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at     TIMESTAMPTZ
);

CREATE INDEX stat_imports_created_at_idx ON stat_imports (created_at DESC);
