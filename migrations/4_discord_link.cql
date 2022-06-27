ALTER TABLE players
    ADD discord_id TEXT;

CREATE MATERIALIZED VIEW players_by_discord_id AS
SELECT *
FROM players
WHERE uuid IS NOT NULL
  AND discord_id IS NOT NULL
PRIMARY KEY ( discord_id, uuid );

CREATE TABLE discords_link
(
    code TEXT PRIMARY KEY,
    uuid UUID
);


