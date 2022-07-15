CONSISTENCY ANY;

DROP MATERIALIZED VIEW statistics_processable;

DROP MATERIALIZED VIEW statistics_processable_ig;

ALTER TABLE statistics
    DROP game_id;

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