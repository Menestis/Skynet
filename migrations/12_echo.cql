CONSISTENCY ANY;

ALTER TABLE servers
    ADD echo_key UUID;

ALTER TABLE players
    ADD echo_enabled BOOLEAN;