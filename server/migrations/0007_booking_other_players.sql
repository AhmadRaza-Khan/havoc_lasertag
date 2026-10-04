-- Other registered players joining a booking (besides the player it's booked for).
-- The booking's num_people covers them too (plus any unregistered guests).
ALTER TABLE bookings ADD COLUMN other_players UUID[] NOT NULL DEFAULT '{}';
