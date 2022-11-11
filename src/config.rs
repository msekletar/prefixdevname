// SPDX-License-Identifier:  MIT

use std::cmp::Ordering;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io;
use std::io::Write;
use std::path::PathBuf;
use std::string::ToString;

use ini::Ini;
use regex::Regex;

use crate::hwaddr_from_event_device;
use crate::util::*;

static NET_SETUP_LINK_CONF_DIR: &str = "/etc/systemd/network/";
static LINK_FILE_PREFIX: &str = "71-net-ifnames-prefix-";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefixedLink {
    pub name: String,
    pub index: u64,
    pub hwaddr: String,
}

impl PrefixedLink {
    pub fn new<T: ToString>(link_name: &T) -> Result<PrefixedLink, Box<dyn Error>> {
        let name = link_name.to_string();
        PrefixedLink::link_name_sane(&name)?;

        lazy_static! {
            static ref RE: Regex = Regex::new(r"([[:alpha:]]+)\d+").unwrap();
        }

        let prefix = match RE.captures(&name) {
            Some(c) => c[1].to_string(),
            None => "".to_string(),
        };

        let i = name.trim_start_matches(&prefix).parse::<u64>()?;

        let config = PrefixedLink {
            name,
            index: i,
            hwaddr: hwaddr_from_event_device()?,
        };

        Ok(config)
    }

    pub fn new_with_hwaddr<T: ToString>(
        link_name: &T,
        hwaddr: &T,
    ) -> Result<PrefixedLink, Box<dyn Error>> {
        let addr = hwaddr_normalize(hwaddr)?;
        let name = link_name.to_string();
        PrefixedLink::link_name_sane(link_name)?;

        lazy_static! {
            static ref RE: Regex = Regex::new(r"([[:alpha:]]+)\d+").unwrap();
        }

        let prefix = match RE.captures(&name) {
            Some(c) => c[1].to_string(),
            None => "".to_string(),
        };
        let i = name.trim_start_matches(&prefix).parse::<u64>()?;

        let config = PrefixedLink {
            name: link_name.to_string(),
            index: i,
            hwaddr: addr,
        };

        Ok(config)
    }

    pub fn link_name_sane<T: ToString>(link_name: &T) -> Result<(), Box<dyn Error>> {
        let name = link_name.to_string();

        if name.is_empty() {
            return Err(From::from("Link name can't be empty string"));
        }

        if name.as_bytes().len() > 16 {
            return Err(From::from("Link name too long"));
        }

        Ok(())
    }

    pub fn link_file_path(&self) -> PathBuf {
        let mut path = PathBuf::from(NET_SETUP_LINK_CONF_DIR);

        path.push(LINK_FILE_PREFIX.to_string() + &self.name + ".link");
        path
    }

    pub fn write_link_file(&self) -> Result<(), Box<dyn Error>> {
        fs::create_dir_all(NET_SETUP_LINK_CONF_DIR)?;

        let path = self.link_file_path();
        let mut link_file = fs::File::create(path)?;

        write!(
            &mut link_file,
            "[Match]\nMACAddress={}\n\n[Link]\nName={}\n",
            self.hwaddr, self.name
        )?;

        Ok(())
    }
}

impl Ord for PrefixedLink {
    fn cmp(&self, other: &PrefixedLink) -> Ordering {
        self.index.cmp(&other.index)
    }
}

impl PartialOrd for PrefixedLink {
    fn partial_cmp(&self, other: &PrefixedLink) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct NetSetupLinkConfig {
    config: HashMap<String, PrefixedLink>,
    links: Vec<PrefixedLink>,
    ifname_prefix: String,
}

impl NetSetupLinkConfig {
    pub fn new_with_prefix<T: ToString>(prefix: &T) -> Self {
        NetSetupLinkConfig {
            config: HashMap::new(),
            links: Vec::new(),
            ifname_prefix: prefix.to_string(),
        }
    }

    pub fn load(&mut self) -> Result<(), Box<dyn Error>> {
        self.enumerate_links_from_udev()?;
        self.enumerate_links_from_files()?;

        // Most links have link file present and are currently known to udev.
        // Hence enumeration from both sources created duplicate entries in the links vector.
        self.links.sort();
        self.links.dedup();

        debug!("Links: {:?}", self.links);

        Ok(())
    }

    pub fn for_hwaddr<T: ToString>(&self, mac: &T) -> Option<PrefixedLink> {
        if let Some(c) = self.config.get(&mac.to_string()) {
            return Some(c.clone());
        }
        None
    }

    pub fn next_link_name(&self) -> Result<String, Box<dyn Error>> {
        if self.links.is_empty() {
            return Ok(format!("{}{}", self.ifname_prefix, "0"));
        }

        let last = self
            .links
            .last()
            .ok_or("Failed to obtain last vector element")?;
        let last_index = last
            .name
            .trim_start_matches(&self.ifname_prefix)
            .parse::<u64>()?;

        Ok(format!(
            "{}{}",
            self.ifname_prefix,
            &(last_index + 1).to_string()
        ))
    }

    fn match_ethernet_links(
        udev_enumerate: &mut libudev::Enumerator,
    ) -> Result<(), Box<dyn Error>> {
        udev_enumerate.match_subsystem("net")?;
        udev_enumerate.match_attribute("type", "1")?;

        Ok(())
    }

    fn enumerate_links_from_udev(&mut self) -> Result<(), Box<dyn Error>> {
        let udev = libudev::Context::new()?;
        let mut enumerate = libudev::Enumerator::new(&udev)?;
        let mut links = Vec::new();

        NetSetupLinkConfig::match_ethernet_links(&mut enumerate)?;

        for device in enumerate.scan_devices()? {
            let name = device
                .sysname()
                .unwrap()
                .to_str()
                .ok_or("Failed to convert from ffi::OsStr to &str");

            if !name?.to_string().starts_with(&self.ifname_prefix) {
                continue;
            }

            // XXX: Move this to its own function and add more devtypes
            match device.devtype() {
                Some(t) => match t.to_str() {
                    Some("vlan") | Some("bond") | Some("bridge") => continue,
                    _ => {}
                },
                None => {}
            }

            let hwaddr = device
                .attribute_value("address")
                .ok_or("Failed to read value of the 'address' sysfs attribute")?
                .to_str()
                .ok_or("Failed to convert from ffi::OsStr to &str");
            links.push(PrefixedLink::new_with_hwaddr(&name?, &hwaddr?)?);
        }

        self.links = links;

        Ok(())
    }

    fn enumerate_links_from_files(&mut self) -> Result<(), Box<dyn Error>> {
        let mut link_files = Vec::new();

        let files = match fs::read_dir(NET_SETUP_LINK_CONF_DIR) {
            Ok(d) => d,
            Err(e) => match e.kind() {
                io::ErrorKind::NotFound => return Ok(()),
                _ => return Err(From::from(e)),
            },
        };

        for f in files {
            let entry = match f {
                Ok(e) => e,
                Err(_) => continue,
            };

            let path = entry.path();
            {
                let name = path
                    .file_name()
                    .ok_or("Failed to obtain filename")?
                    .to_str()
                    .ok_or("Failed to convert OsStr to String")?;

                if !name.starts_with(LINK_FILE_PREFIX) || !name.ends_with(".link") {
                    continue;
                }
            }

            link_files.push(path);
        }

        for l in &link_files {
            let conf = Ini::load_from_file(l)?;
            let match_section = conf
                .section(Some("Match".to_owned()))
                .ok_or("Failed to parse link file, [Match] section not found")?;
            let link_section = conf
                .section(Some("Link".to_owned()))
                .ok_or("Failed to parse link file, [Link] section not found")?;

            let mac = match_section.get("MACAddress").ok_or("Failed to parse link file, \"MACAddress\"' option not present in the [Link] section")?;
            let name = link_section.get("Name").ok_or(
                "Failed to parse link file, \"Name\" option not present in the [Link] section",
            )?;

            if !name.starts_with(&self.ifname_prefix) {
                warn!("Unexpected link name");
                continue;
            }

            let hwaddr = mac;

            self.config
                .insert(hwaddr.to_string(), PrefixedLink::new(&name)?);
            self.links
                .push(PrefixedLink::new_with_hwaddr(&name, &hwaddr)?);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    enum UMockdevTestbed {}
    #[link(name = "umockdev")]
    extern "C" {
        fn umockdev_testbed_new() -> *mut UMockdevTestbed;
        fn umockdev_testbed_add_from_string(
            testbed: *mut UMockdevTestbed,
            device_description: *mut c_char,
            err: *mut *mut u8,
        );
    }

    use std::env;
    use std::ffi::CString;
    use std::os::raw::c_char;
    use std::path::Path;

    use super::*;

    #[test]
    fn prefixed_link_new() {
        let config = PrefixedLink::new_with_hwaddr(&"net0", &"ff:ff:ff:ff:ff:ff");
        assert!(config.is_ok());
    }

    #[test]
    fn prefixed_link_name_empty() {
        let config = PrefixedLink::new_with_hwaddr(&"", &"ff:ff:ff:ff:ff:ff");
        assert!(config.is_err());
    }

    #[test]
    fn prefixed_link_name_long() {
        let config =
            PrefixedLink::new_with_hwaddr(&"neeeeeeeeeeeeeeeeeeeeeeeeeet0", &"ff:ff:ff:ff:ff:ff");
        assert!(config.is_err());
    }

    #[test]
    fn prefixed_link_name_invalid() {
        let config = PrefixedLink::new_with_hwaddr(&"1net0", &"ff:ff:ff:ff:ff:ff");
        assert!(config.is_err());
    }

    #[test]
    #[should_panic]
    fn prefixed_link_invalid_hwaddr() {
        let _config = PrefixedLink::new_with_hwaddr(&"net0", &"de:ad:be:ee:ff:xx").unwrap();
    }

    #[test]
    #[should_panic]
    fn prefixed_link_hwaddr_too_long() {
        let _config = PrefixedLink::new_with_hwaddr(&"net0", &"ff:ff:ff:ff:ff:ff:ff").unwrap();
    }

    #[test]
    fn prefixed_link_hwaddr_all_caps() {
        let config = PrefixedLink::new_with_hwaddr(&"net0", &"FF:FF:FF:FF:FF:FF");
        assert!(config.is_ok());
    }

    fn mock_sysfs() -> Result<(), Box<dyn Error>> {
        use std::io::prelude::*;
        use std::ptr;
        let mut err: *mut u8 = ptr::null_mut();

        let mut net0 = fs::File::open("test/net0.mockdev").unwrap();
        let mut net0_device_description = String::new();

        let mut eth0 = fs::File::open("test/eth0.mockdev").unwrap();
        let mut eth0_device_description = String::new();

        net0.read_to_string(&mut net0_device_description).unwrap();
        eth0.read_to_string(&mut eth0_device_description).unwrap();

        // make eth0 newly discovered NIC
        env::set_var("DEVPATH", "/devices/pci0000:00/0000:00:03.0/net/eth0");
        unsafe {
            let test_bed = umockdev_testbed_new();
            umockdev_testbed_add_from_string(
                test_bed,
                CString::new(net0_device_description).unwrap().into_raw(),
                &mut err,
            );
            umockdev_testbed_add_from_string(
                test_bed,
                CString::new(eth0_device_description).unwrap().into_raw(),
                &mut err,
            );
        }

        Ok(())
    }

    #[test]
    #[ignore = "Test requires special environment - use make check"]
    fn prefixed_link_name() {
        mock_sysfs().unwrap();

        let prefixed_link = PrefixedLink::new(&"net1").unwrap();
        assert_eq!(prefixed_link.name, "net1");
    }

    #[test]
    #[ignore = "Test requires special environment - use make check"]
    fn prefixed_link_hwaddr() {
        mock_sysfs().unwrap();

        let prefixed_link = PrefixedLink::new(&"net1").unwrap();
        assert_eq!(prefixed_link.hwaddr.to_string(), "52:54:00:1C:08:B7");
    }

    #[test]
    #[ignore = "Test requires special environment - use make check"]
    fn prefixed_link_link_file_path() {
        mock_sysfs().unwrap();

        let prefixed_link = PrefixedLink::new(&"net1").unwrap();
        assert_eq!(
            prefixed_link.link_file_path().as_path(),
            Path::new("/etc/systemd/network/71-net-ifnames-prefix-net1.link")
        );
    }

    #[test]
    #[ignore = "Test requires special environment - use make check"]
    fn net_setup_link_config_sysfs_only() {
        mock_sysfs().unwrap();

        let mut net_setup_link_config = NetSetupLinkConfig::new_with_prefix(&"net");
        net_setup_link_config.load().unwrap();

        assert_eq!("net1", net_setup_link_config.next_link_name().unwrap());
    }

    #[test]
    #[ignore = "Test requires special environment - use make check"]
    fn xx_net_setup_link_config_mix() {
        mock_sysfs().unwrap();

        let c1 = PrefixedLink::new_with_hwaddr(&"net1", &"FF:FF:FF:FF:FF:AA").unwrap();
        let c2 = PrefixedLink::new_with_hwaddr(&"net2", &"FF:FF:FF:FF:FF:BB").unwrap();
        let c3 = PrefixedLink::new_with_hwaddr(&"net3", &"FF:FF:FF:FF:FF:CC").unwrap();

        c1.write_link_file().unwrap();
        c2.write_link_file().unwrap();
        c3.write_link_file().unwrap();

        let mut net_setup_link_config = NetSetupLinkConfig::new_with_prefix(&"net");
        net_setup_link_config.load().unwrap();

        assert_eq!("net4", net_setup_link_config.next_link_name().unwrap());
    }
}
