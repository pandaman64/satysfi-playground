#!/bin/sh

if [ -z "$1" ]
then
  echo "no arguments supplied"
  exit 1
fi

CONTAINER=$(docker create pandaman64/satysfi-playground)
docker cp "$1" "$CONTAINER:/tmp/input.saty" > /dev/null
docker start -a "$CONTAINER"
docker cp "$CONTAINER:/tmp/output.pdf" output.pdf > /dev/null
docker rm "$CONTAINER" > /dev/null

