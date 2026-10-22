-- SkipCash credentials, entered by admins in Settings instead of the server's environment.
-- At most one row; no row means online payment is off.
CREATE TABLE skipcash_credentials (
    id           BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (id),
    environment  TEXT NOT NULL CHECK (environment IN ('sandbox', 'production')),
    -- "Client ID" (merchant portal → Credentials); authorizes reading a payment.
    client_id    TEXT NOT NULL,
    -- The private key's ID and secret; the secret signs payment requests.
    key_id       TEXT NOT NULL,
    key_secret   TEXT NOT NULL,
    -- SkipCash signs its webhook calls with this.
    webhook_key  TEXT NOT NULL,
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
