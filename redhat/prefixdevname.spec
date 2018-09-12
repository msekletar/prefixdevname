Name:           prefixdevname
Version:        0.1.0
Release:        1%{?dist}
Summary:        Udev helper utility that provides network interface naming using user defined prefix

License:        MIT
URL:            https://www.github.com/msekletar/prefixdevname
Source0:        https://github.com/msekletar/%{name}/releases/download/v%{version}/%{name}-%{version}.tar.gz
Source1:        %{name}-%{version}-vendor.tar.gz

ExclusiveArch: %{rust_arches}

BuildRequires:  rust-toolset-1.26
BuildRequires:  git
BuildRequires:  systemd-devel

%description
This package provides udev helper utility that tries to consistently name all ethernet NICs using
user defined prefix (e.g. net.ifnames.prefix=net produces NIC names net0, net1, ...). Utility is
called from udev rule and it determines NIC name and writes out configuration file for udev's
net_setup_link built-in (e.g. /etc/systemd/network/71-net-ifnames-prefix-net0.link).

%prep
%autosetup -S git_am
%cargo_prep -V 1

%build
%cargo_build

%install
%make_install

%files
%defattr(-,root,root,-)
%license LICENSE
%doc README.md
%{_prefix}/lib/udev/%{name}
%{_prefix}/lib/udev/rules.d/*.rules
%dir %{_prefix}/lib/dracut/modules.d/71%{name}
%{_prefix}/lib/dracut/modules.d/71%{name}/*
%dir %{_prefix}/lib/dracut/modules.d/71%{name}-tools
%{_prefix}/lib/dracut/modules.d/71%{name}-tools/*

%changelog
* Wed Aug 08 2018 Michal Sekletar <msekleta@redhat.com>
- initial package
