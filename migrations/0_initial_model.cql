CONSISTENCY ALL;
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
    current_ip       INET,             //The current player ip

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


CREATE TABLE sessions
(
    uuid  UUID,
    start TIMESTAMP,
    end   TIMESTAMP,
    ip    INET,
    PRIMARY KEY ( uuid, start )
);


CREATE TABLE statistics
(
    uuid    UUID,
    session UUID,
    key     TEXT,
    value   INT,
    PRIMARY KEY ( uuid, session, key )
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
    label       TEXT,             //Name
    kind        TEXT,
    state       TEXT,             //Current serer state (STARTING, RUNNING, STOPPED)
    description TEXT,             //Anything you want
    properties  MAP<TEXT, TEXT>
);

CREATE MATERIALIZED VIEW servers_by_ip AS
SELECT *
FROM servers
WHERE id IS NOT NULL
  AND ip IS NOT NULL
PRIMARY KEY ( ip, id );

CREATE TABLE servers_kinds
(
    name TEXT PRIMARY KEY,
    image TEXT,
    permissions MAP<TEXT, FROZEN<LIST<TEXT>>>,
    autoscale TEXT
);

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