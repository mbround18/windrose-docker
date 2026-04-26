#!/bin/bash

echo "-- Starting Windrose Docker Container --"

echo "-- SteamCMD Preflight --"

/home/steam/steamcmd/steamcmd.sh +login anonymous +app_info_update 1 +quit

echo "-- Installing Windrose --"

gsm-windrose install

echo "-- Windrose Installation Complete --"

mkdir -p /home/steam/windrose/logs/
touch /home/steam/windrose/logs/server.log
touch /home/steam/windrose/logs/server.err

gsm-windrose start

echo "-- Windrose Started --"

# Wait for ServerDescription.json to be created
SERVER_DESCRIPTION_FILE="/home/steam/windrose/R5/ServerDescription.json"
MAX_WAIT_TIME=120 # seconds
WAIT_INTERVAL=5   # seconds
ELAPSED_TIME=0

echo "Waiting for $SERVER_DESCRIPTION_FILE to be created..."

while [ ! -f "$SERVER_DESCRIPTION_FILE" ] && [ "$ELAPSED_TIME" -lt "$MAX_WAIT_TIME" ]; do
  echo "File not found yet. Waiting $WAIT_INTERVAL seconds..."
  sleep "$WAIT_INTERVAL"
  ELAPSED_TIME=$((ELAPSED_TIME + WAIT_INTERVAL))
done

if [ ! -f "$SERVER_DESCRIPTION_FILE" ]; then
  echo "Error: $SERVER_DESCRIPTION_FILE was not created within $MAX_WAIT_TIME seconds."
  exit 1
fi

echo "$SERVER_DESCRIPTION_FILE found. Proceeding with monitor."

exec gsm-windrose monitor



