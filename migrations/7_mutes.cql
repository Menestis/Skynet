CREATE TABLE mutes_logs
(
    id        UUID, //Sanction id, provided to the user on the mute message
    start     TIMESTAMP,
    end       TIMESTAMP,
    target    UUID, //The player the sanction is applied on (Null if none)
    issuer    UUID, //The player that created the sanction (Null if none)
    reason    TEXT, //The reason (Null if none),
    PRIMARY KEY ( id)
);


ALTER TABLE players
    ADD mute TEXT;

