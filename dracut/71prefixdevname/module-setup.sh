#!/bin/bash

# Make sure we always include generated link files in initrd
check() {
    return 0
}

depends() {
  echo systemd
}

install() {
    orig_shopt="$(shopt -p nullglob)"
    shopt -q -u nullglob

    inst_multiple -H -o /etc/systemd/network/71-net-ifnames-prefix-*.link

    eval "$orig_shopt"
}
