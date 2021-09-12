#!/bin/sh

cat .pid | xargs kill
rm .pid
