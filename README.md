# Windrose Docker Server

This project provides a Docker setup for the Windrose Dedicated Server, along with a custom Rust CLI tool (`gsm-windrose`) to manage its lifecycle (install, start, stop, restart, update, monitor).

## Environment Variables

The behavior of the `gsm-windrose` CLI and the server configuration can be controlled via the following environment variables:

| Environment Variable            | Description                                                                 | Default Value                    |
| :------------------------------ | :-------------------------------------------------------------------------- | :------------------------------- |
| `APP_ID`                        | Steam App ID for the Windrose server.                                       | `4129620`                        |
| `INSTALL_PATH`                  | The absolute path where the server files will be installed.                 | `/home/steam/windrose`           |
| `NAME`                          | The display name for the dedicated server instance.                         | `Windrose Dedicated Server`      |
| `WINDROSE_LAUNCH_ARGS`          | Additional command-line arguments to pass to the Windrose server executable.| (empty string)                   |
| `AUTO_UPDATE`                   | Set to `true` to enable automatic server updates.                           | `false`                          |
| `AUTO_UPDATE_CRON`              | Cron schedule for automatic updates (e.g., `0 0 * * *` for daily at midnight). | `0 0 * * *` (daily)              |
| `WINDROSE_SERVER_NAME`          | Sets the in-game display name for the server.                               | (server default, usually empty)  |
| `WINDROSE_MAX_PLAYERS`          | Sets the maximum number of players allowed on the server.                   | (server default, usually `8`)    |
| `WINDROSE_REGION`               | Sets the user-selected region for the server.                               | (server default, usually empty)  |
| `WINDROSE_DIRECT_CONNECTION`    | Set to `true` to enable direct connection.                                  | (server default, usually `false`)|
| `WINDROSE_DIRECT_PORT`          | Sets the direct connection server port if direct connection is enabled.     | (server default, usually `-1`)   |
| `WINDROSE_PASSWORD`             | Sets the password required to join the server. Setting this enables password protection. | (empty string)                   |

## Usage

(Further usage instructions would go here, explaining how to use `compose.yml`, `entrypoint.sh`, and the `gsm-windrose` commands.)
