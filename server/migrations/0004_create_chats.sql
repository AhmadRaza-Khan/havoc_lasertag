-- When the player last had the app connected (chat presence). NULL = never connected.
ALTER TABLE players ADD COLUMN last_seen_at TIMESTAMPTZ;

-- One private conversation per pair of players. The pair is stored ordered
-- (player_a < player_b) so each pair has exactly one row.
CREATE TABLE chat_conversations (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    player_a        UUID NOT NULL REFERENCES players (id) ON DELETE CASCADE,
    player_b        UUID NOT NULL REFERENCES players (id) ON DELETE CASCADE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_message_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (player_a < player_b),
    UNIQUE (player_a, player_b)
);

CREATE INDEX chat_conversations_player_b_idx ON chat_conversations (player_b);

CREATE TABLE chat_messages (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id UUID NOT NULL REFERENCES chat_conversations (id) ON DELETE CASCADE,
    sender_id       UUID NOT NULL REFERENCES players (id) ON DELETE CASCADE,
    body            TEXT NOT NULL CHECK (char_length(body) BETWEEN 1 AND 2000),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Set when the recipient reads it
    read_at         TIMESTAMPTZ
);

CREATE INDEX chat_messages_conversation_idx ON chat_messages (conversation_id, created_at DESC, id DESC);
CREATE INDEX chat_messages_unread_idx ON chat_messages (conversation_id, sender_id) WHERE read_at IS NULL;
