INSERT INTO api_groups(name, permissions)
VALUES ('proxy',
        {'proxy-login', 'proxy-pre-login', 'proxy-close-session', 'get-online-players', 'proxy-ping', 'move-player', 'ban-player', 'get-all-servers', 'get-player', 'player-transaction', 'update-player-groups', 'player-inventory-transaction', 'set-server-state', 'proxy-update-playercount', 'webhook', 'mute-player'});

INSERT INTO api_groups(name, permissions)
VALUES ('server',
        {'server-login', 'ban-player','player-stats', 'move-player', 'create-server', 'delete-server', 'get-all-servers', 'player-transaction', 'player-inventory-transaction', 'set-server-state', 'get-onlinecount', 'get-stats', 'create-discord-link', 'webhook', 'set-server-description', 'broadcast', 'server-update-playercount', 'get-player'});

INSERT INTO api_groups(name, permissions)
VALUES ('discord', {'complete-discord-link', 'get-onlinecount', 'get-stats', 'delete-discord-link'});
