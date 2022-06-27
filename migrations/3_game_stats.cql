CONSISTENCY ANY;

CREATE TABLE games
(
    id         UUID PRIMARY KEY,
    name       TEXT,
    kind       TEXT,
    date       TIMESTAMP,
    hosts      LIST<UUID>, //Nullable
    properties MAP<TEXT, TEXT>
);

ALTER TABLE statistics
    ADD game_id TEXT; //Nullable

ALTER TABLE statistics
    ADD game_kind TEXT; //Nullable

CREATE TABLE leaderboards
(
    name        TEXT PRIMARY KEY,
    label       TEXT,
    leaderboard LIST<TEXT>,
    rules       TEXT,
);

DROP MATERIALIZED VIEW statistics_processable;

CREATE MATERIALIZED VIEW statistics_processable AS
SELECT *
FROM statistics
WHERE player IS NOT NULL
  AND timestamp IS NOT NULL
  AND key IS NOT NULL
  AND server_kind IS NOT NULL
  AND game_kind IS NOT NULL
  AND server_id IS NOT NULL
  AND session IS NOT NULL
  AND game_id IS NOT NULL
PRIMARY KEY (key, timestamp, game_kind, player, server_kind, server_id, session);

CREATE MATERIALIZED VIEW statistics_processable_ig AS
SELECT *
FROM statistics
WHERE player IS NOT NULL
  AND timestamp IS NOT NULL
  AND key IS NOT NULL
  AND game_kind IS NOT NULL
  AND server_kind IS NOT NULL
  AND server_id IS NOT NULL
  AND session IS NOT NULL
PRIMARY KEY (key, game_kind, timestamp, player, server_kind, server_id, session);


