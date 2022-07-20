CONSISTENCY ANY;

CREATE TABLE sanctions_board
(
    category  TEXT PRIMARY KEY,
    label TEXT,
    sanctions LIST<TEXT>
);

CREATE TABLE sanctions_states
(
    player   UUID,
    category TEXT,
    value    INT,
    PRIMARY KEY ((player, category))
);
