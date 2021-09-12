#!/bin/sh

echo $(pwd)

npx http-server -c-1 -p 9000 www/ &

echo $! > .pid
