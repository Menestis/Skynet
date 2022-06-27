INSERT INTO api_groups(name, permissions)
VALUES ('proxy',
        {'proxy-login', 'proxy-pre-login', 'proxy-close-session', 'get-online-players', 'proxy-ping', 'move-player', 'ban-player', 'get-all-servers', 'get-player', 'player-transaction', 'update-player-groups', 'player-inventory-transaction', 'set-server-state', 'proxy-update-playercount'});

INSERT INTO api_groups(name, permissions)
VALUES ('server',
        {'server-login', 'player-stats', 'move-player', 'create-server', 'delete-server', 'get-all-servers', 'player-transaction', 'player-inventory-transaction', 'set-server-state', 'get-onlinecount', 'get-stats', 'create-discord-link'});

INSERT INTO api_groups(name, permissions)
VALUES ('discord', {'complete-discord-link', 'get-onlinecount', 'get-stats'});
