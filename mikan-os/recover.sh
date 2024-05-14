#!/bin/bash -ex

DIR=$(dirname $0)

# kill qemu
PIDS=$(ps ax | grep qemu | grep -v grep | awk '{ print $1 }')
if [ -n "$PIDS" ]; then
	kill -9 $PIDS
fi

# recover OVMF_VARS.fd
git checkout HEAD $DIR/../devenv/OVMF_VARS.fd
