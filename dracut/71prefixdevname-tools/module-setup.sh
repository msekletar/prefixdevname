#!/bin/bash

# Include the prefixdevname tools only if this was explicitly requested
check() {
    return 255
}

install() {
    inst /usr/lib/udev/prefixdevname
    inst_rules 71-prefixdevname.rules
}
