-- "Havoc Admin": the player account staff chat as. Messages sent from the admin panel's Chats
-- page come from it, so players see them in the app like any chat. Nobody can sign in as it
-- (the login lookup skips it and the hash matches no password); it's hidden from the panel's
-- Players list. The id is fixed: `players::HAVOC_ADMIN_ID`.
INSERT INTO players (id, full_name, playername, password_hash)
VALUES ('00000000-0000-0000-0000-00000000ad01', 'Havoc Admin', 'admin', '!no-login')
ON CONFLICT DO NOTHING;
