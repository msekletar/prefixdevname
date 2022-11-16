# prefixdevname | [![Rust CI](https://github.com/msekletar/prefixdevname/actions/workflows/rust-ci.yml/badge.svg)](https://github.com/msekletar/prefixdevname/actions/workflows/rust-ci.yml) [![codecov](https://codecov.io/gh/msekletar/prefixdevname/branch/main/graph/badge.svg?token=XFZXPIS5I9)](https://codecov.io/gh/msekletar/prefixdevname)

Simple udev helper that let's you define your own prefix used for NIC naming.

## License

MIT

## Building from source

prefixdevname is Rust project hence we use cargo as our build tool, however, the project also includes Makefile in order
to simplify some common tasks. For example, unit tests mock sysfs using libumockdev in unprivileged user namespace and
this requires the manual configuration which is handled by make's check target.

```sh
make
make check
sudo make install
```

## Contributing

In case you find a problem with prefixdevname please file an issue on Github. Of course feel free to send PRs as well.

## Installation and usage

Prefixdevname (name inspired by Dell's biosdevname) requires very minimal setup. End user needs to install the package
and specify the desired prefix that should be used for NIC naming on the kernel command line. For example, these are
the setup steps for Fedora,

```sh
dnf install prefixdevname
grubby --update-kernel=$(grubby --default-kernel) --args="net.ifnames.prefix=net"
reboot
```

prefixdevname is spawned every time new networking hardware appears and we try to figure out what should be the next possible
device name respecting our enumeration. We assign device name in the form "\<PREFIX\>\<INDEX\>", e.g. net2 in case that net0 and
net1 are already present. The tool then generates the new .link file in /etc/systemd/network directory that applies the name
to the interface with the MAC address that just appeared. Hence the configuration is persistent across reboots (it would make
little sense otherwise).

## Limitations

After reboot the machine will name all Ethernet network devices using the "net" prefix, e.g. net0.
If the renaming is done on the already deployed system you have to make sure that your network configuration
tool is aware of the new naming scheme (e.g. on Fedora one would have to adjust ifcfg files accordingly). This
is not an issue when naming scheme is already used at system installation time and network configuration
is generated using prefix based names.

User-defined prefix must be ASCII string that matches following regular expression, [[:alpha:]]+ and must be shorter
than 15 characters.

Another limitation is that your prefix can not conflict with any other well-known prefix used for NIC naming on Linux.
Specifically you can't use any of the following prefixes:

* eth
* eno
* ens
* em

After adding new network hardware that got renamed it is highly advised to run "dracut -f" in order to make sure that
newly generated .link configuration files are also included in the initramfs image. This repository also contains very
minimal dracut module that handles inclusion of .link files to the initramfs image.
