ALTER TABLE players
    ADD waiting_move_to TEXT;

CREATE MATERIALIZED VIEW players_by_waiting_move_to AS
SELECT *
FROM players
WHERE uuid IS NOT NULL
  AND session IS NOT NULL
  AND proxy IS NOT NULL
  AND waiting_move_to IS NOT NULL
PRIMARY KEY (waiting_move_to, uuid);

CREATE MATERIALIZED VIEW players_by_server AS
SELECT *
FROM players
WHERE uuid IS NOT NULL
  AND server IS NOT NULL
PRIMARY KEY (server, uuid);


CREATE MATERIALIZED VIEW servers_by_kinds AS
SELECT *
FROM servers
WHERE id IS NOT NULL
  AND kind IS NOT NULL
PRIMARY KEY ( kind, id );