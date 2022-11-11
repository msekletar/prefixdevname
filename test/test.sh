#!/bin/bash
set -e

tmpdir=$(mktemp -d)

# setup temporary /etc
mount --bind "$tmpdir" /etc/systemd/network

# run the actual test suite
umockdev-wrapper cargo test -- --include-ignored --test-threads=1

# cleanup
umount /etc/systemd/network
rm -rf /tmp/umockdev.*
rm -rf "$tmpdir"

