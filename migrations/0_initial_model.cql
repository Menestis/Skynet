CONSISTENCY ANY;
CREATE TABLE groups
(
    name        TEXT PRIMARY KEY, //group name
    power       INT,              //The highest is the stronger
    prefix      TEXT,
    suffix      TEXT,
    permissions LIST<TEXT>,       //List of permissions, add a '-' at the beginning to remove the permission
);

INSERT INTO groups (name, power)
VALUES ('Default', 0);

CREATE TABLE api_groups
(
    name        TEXT PRIMARY KEY,
    permissions SET<TEXT>
);

INSERT INTO api_groups(name, permissions)
VALUES ('proxy',
        {'proxy-login', 'proxy-pre-login', 'proxy-close-session', 'get-online-players', 'proxy-ping', 'move-player', 'ban-player', 'get-all-servers', 'get-player', 'player-transaction', 'update-player-groups', 'player-inventory-transaction', 'set-server-state', 'proxy-update-playercount'});

INSERT INTO api_groups(name, permissions)
VALUES ('server',
        {'server-login', 'player-stats', 'move-player', 'create-server', 'delete-server', 'get-all-servers', 'player-transaction', 'player-inventory-transaction', 'set-server-state'});

CREATE TABLE api_keys
(
    key   UUID PRIMARY KEY,
    group TEXT //Null group means admin privileges BE CAREFULL
);

CREATE TABLE players
(
    uuid             UUID PRIMARY KEY, //Player uuid
    username         TEXT,             //Updated each time the player successfully joins

    prefix           TEXT,             //Always has priority on the rank
    suffix           TEXT,             //^

    locale           TEXT,             //Language

    groups           LIST<TEXT>,       //groups
    permissions      LIST<TEXT>,       //And permissions

    currency         INT,              //The money he has
    premium_currency INT,              //The premium money he has
    inventory        MAP<TEXT, INT>,   //Inventory (boosts, cosmetics, etc)
    properties       MAP<TEXT, TEXT>,

    //If the player is online
    proxy            UUID,             //Current proxy the player is connected to
    server           UUID,             //Current server the player is connected to
    session          UUID,             //The current player session

    //Friends
    friends          SET<UUID>,
    friend_policy    TEXT,             //EVERYONE, NOBODY, FRIENDS_OF_FRIENDS

    //Blocked users
    blocked          SET<UUID>,

    //Bans
    ban_reason       TEXT,
    ban              UUID,
);

CREATE MATERIALIZED VIEW players_by_username AS
SELECT *
FROM players
WHERE uuid IS NOT NULL
  AND username IS NOT NULL
PRIMARY KEY ( username, uuid );

CREATE MATERIALIZED VIEW players_by_session AS
SELECT *
FROM players
WHERE uuid IS NOT NULL
  AND session IS NOT NULL
PRIMARY KEY ( session, uuid );

CREATE MATERIALIZED VIEW players_by_server AS
SELECT *
FROM players
WHERE uuid IS NOT NULL
  AND server IS NOT NULL
PRIMARY KEY ( server, uuid );

CREATE TABLE sessions
(
    id      UUID PRIMARY KEY,
    player  UUID,
    start   TIMESTAMP,
    end     TIMESTAMP,
    ip      INET,
    version TEXT,
    brand   TEXT,
    mods    MAP<TEXT,TEXT>,
);

CREATE MATERIALIZED VIEW sessions_by_player AS
SELECT *
FROM sessions
WHERE id IS NOT NULL
  AND player IS NOT NULL
PRIMARY KEY ( player, id );


CREATE MATERIALIZED VIEW sessions_by_ip AS
SELECT *
FROM sessions
WHERE id IS NOT NULL
  AND ip IS NOT NULL
PRIMARY KEY ( ip, id );

CREATE TABLE statistics
(
    player      UUID,
    session     UUID,
    timestamp   TIMESTAMP,
    server_kind TEXT,
    server_id   UUID,
    key         TEXT,
    value       INT,
    PRIMARY KEY ( player, session, timestamp, server_id, server_kind, key )
);

CREATE MATERIALIZED VIEW statistics_processable AS
SELECT *
FROM statistics
WHERE player IS NOT NULL
  AND timestamp IS NOT NULL
  AND server_kind IS NOT NULL
  AND key IS NOT NULL
  AND server_id IS NOT NULL
  AND session IS NOT NULL
PRIMARY KEY (player, server_kind, key, server_id, session, timestamp);


-- CREATE MATERIALIZED VIEW statistics_by_player_and_server_kind AS
-- SELECT *
-- FROM statistics
-- WHERE player IS NOT NULL
--   AND session IS NOT NULL
--   AND server_kind IS NOT NULL
--   AND server_id IS NOT NULL
--   AND key IS NOT NULL
--   AND timestamp IS NOT NULL
-- PRIMARY KEY (player, server_kind, key, server_id, session, timestamp);
--
-- CREATE MATERIALIZED VIEW statistics_by_key AS
-- SELECT *
-- FROM statistics
-- WHERE player IS NOT NULL
--   AND session IS NOT NULL
--   AND server_kind IS NOT NULL
--   AND server_id IS NOT NULL
--   AND key IS NOT NULL
--   AND timestamp IS NOT NULL
-- PRIMARY KEY (player, key, server_kind, timestamp, session, server_id);
--
-- CREATE MATERIALIZED VIEW statistics_by_timestamp AS
-- SELECT *
-- FROM statistics
-- WHERE player IS NOT NULL
--   AND session IS NOT NULL
--   AND server_kind IS NOT NULL
--   AND server_id IS NOT NULL
--   AND key IS NOT NULL
--   AND timestamp IS NOT NULL
-- PRIMARY KEY (timestamp, player, server_kind, key, session, server_id);

CREATE TABLE anticheat_logs
(
    player  UUID,
    session UUID,
    check   TEXT,
    value   INT,
    PRIMARY KEY ( player, session, check )
);


CREATE TABLE bans_logs
(
    id     UUID, //Sanction id, provided to the user on the ban screen
    start  TIMESTAMP,
    end    TIMESTAMP,
    target UUID, //The player the sanction is applied on (Null if none)
    ip     INET, //The ip the sanction is applied on (Null if none)
    issuer UUID, //The player that created the sanction (Null if none)
    reason TEXT, //The reason (Null if none),
    PRIMARY KEY (id)
);


CREATE MATERIALIZED VIEW bans_logs_by_target AS
SELECT *
FROM bans_logs
WHERE id IS NOT NULL
  AND target IS NOT NULL
PRIMARY KEY ( target, id );

CREATE MATERIALIZED VIEW bans_logs_by_issuer AS
SELECT *
FROM bans_logs
WHERE id IS NOT NULL
  AND issuer IS NOT NULL
PRIMARY KEY ( issuer, id );

CREATE MATERIALIZED VIEW bans_logs_by_ip AS
SELECT *
FROM bans_logs
WHERE id IS NOT NULL
  AND ip IS NOT NULL
PRIMARY KEY ( ip, id );


CREATE TABLE ip_bans
(
    ip        INET PRIMARY KEY,
    reason    TEXT,
    date      TIMESTAMP,
    end       TIMESTAMP,
    ban       UUID,
    automated BOOLEAN,
);


CREATE TABLE servers
(
    id          UUID PRIMARY KEY, //Server unique id
    ip          INET,
    key         UUID,
    label       TEXT,             //Name
    kind        TEXT,
    state       TEXT,             //Current serer state (STARTING, RUNNING, STOPPED)
    description TEXT,             //Anything you want
    properties  MAP<TEXT, TEXT>
);

CREATE MATERIALIZED VIEW servers_by_label AS
SELECT *
FROM servers
WHERE id IS NOT NULL
  AND label IS NOT NULL
PRIMARY KEY ( label, id );

CREATE MATERIALIZED VIEW servers_by_key AS
SELECT *
FROM servers
WHERE id IS NOT NULL
  AND key IS NOT NULL
PRIMARY KEY ( key, id );

CREATE TABLE servers_logs
(
    id          UUID PRIMARY KEY, //Server unique id
    label       TEXT,             //Name
    kind        TEXT,
    description TEXT,             //Anything you want
    properties  MAP<TEXT, TEXT>
);


CREATE TABLE servers_kinds
(
    name        TEXT PRIMARY KEY,
    image       TEXT,
    permissions MAP<TEXT, FROZEN<LIST<TEXT>>>,
    autoscale   TEXT
);

INSERT INTO servers_kinds (name, image, permissions, autoscale)
VALUES ('proxy', 'registry.aspaku.com/skynet/proxy', {}, null);

INSERT INTO servers_kinds (name, image, permissions, autoscale)
VALUES ('lobby', 'registry.aspaku.com/skynet/lobby', {}, null);

//Simple key value pair
CREATE TABLE settings
(
    key   TEXT PRIMARY KEY,
    value TEXT
);


//Message of the day
INSERT INTO settings (key, value)
VALUES ('motd', 'Default MOTD');

//Maintenance, only aspaku.maintenance.bypass is allowed
INSERT INTO settings (key, value)
VALUES ('maintenance', 'false');

//Server slots (across all bungees)
INSERT INTO settings (key, value)
VALUES ('slots', '400');