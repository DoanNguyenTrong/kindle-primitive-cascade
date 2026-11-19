#!/bin/sh
# Name: Primitive Cascade
# Author: Primitive AI
# DontUseFBInk

app=/mnt/us/cascade/bin/cascade-kindle
[ -x "$app" ] || exit 1
work=$(mktemp -d /var/tmp/primitive-cascade.XXXXXX) || exit 1
trap 'rm -rf "$work"' EXIT
cp "$app" "$work/run" && chmod 755 "$work/run" || exit 1
cd / || exit 1
"$work/run"
