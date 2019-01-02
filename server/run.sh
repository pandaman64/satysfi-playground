#!/bin/sh

if [ -z "$1" -o -z "$2" ]
then
  echo "not enough arguments"
  exit 1
fi

CONTAINER=$(docker create pandaman64/satysfi-playground)
docker cp "$1" "$CONTAINER:/tmp/input.saty" > /dev/null
timeout -sKILL 30s docker start -a "$CONTAINER"
docker cp "$CONTAINER:/tmp/output.pdf" "$2" > /dev/null
docker rm "$CONTAINER" > /dev/null

