#!/bin/bash

# Make sure we always include generated link files in initrd
check() {
    return 0
}

install() {
    orig_shopt="$(shopt -p nullglob)"
    shopt -q -u nullglob

    if dracut_module_included "systemd"; then
        inst_multiple -H -o /etc/systemd/network/71-net-ifnames-prefix-*.link
    fi

    eval "$orig_shopt"
}
