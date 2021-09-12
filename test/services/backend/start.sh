#!/bin/sh

npm install
npm start &

echo $! > .pid
