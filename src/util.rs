// SPDX-License-Identifier:  MIT

use regex::Regex;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

extern crate libudev;
use libudev::Device;

use crate::sema::Semaphore;

pub fn rename_needed(ifname: &str, prefix: &str) -> Result<bool, Box<dyn Error>> {
    let re: Regex = Regex::new(&format!("{}\\d+", prefix)).unwrap();

    Ok(!re.is_match(ifname))
}

pub fn event_device_name() -> String {
    env::var("INTERFACE").unwrap_or_else(|_| "".to_string())
}

pub fn event_device_virtual() -> bool {
    let devpath = env::var("DEVPATH").unwrap_or_else(|_| "".to_string());

    devpath.starts_with("/devices/virtual")
}

pub fn hwaddr_valid<T: ToString>(hwaddr: &T) -> bool {
    use std::num::ParseIntError;

    let hwaddr_length_as_str = 17;
    let addr = hwaddr.to_string();

    if !addr.is_ascii() {
        return false;
    }

    if addr.len() != hwaddr_length_as_str {
        return false;
    }

    let bytes: Vec<Result<u8, ParseIntError>> = addr
        .split(|c| c == ':' || c == '-')
        .map(|s| u8::from_str_radix(s, 16))
        .collect();

    for b in bytes {
        if b.is_err() {
            return false;
        }
    }

    true
}

pub fn hwaddr_normalize<T: ToString>(hwaddr: &T) -> Result<String, Box<dyn Error>> {
    let mut addr = hwaddr.to_string();

    if !hwaddr_valid(&addr) {
        return Err(From::from("Failed to parse MAC address"));
    }

    if addr.find('-').is_some() {
        addr = addr.replace('-', ":")
    }

    addr.make_ascii_uppercase();
    Ok(addr)
}

pub fn hwaddr_from_event_device() -> Result<String, Box<dyn Error>> {
    let udev = libudev::Context::new()?;
    let devpath = env::var("DEVPATH")?;
    let mut syspath = "/sys".to_string();

    syspath.push_str(&devpath);

    let attr = Device::from_syspath(&udev, &PathBuf::from(syspath))?
        .attribute_value("address")
        .ok_or("Failed to get MAC Address")?
        .to_owned();
    let addr = hwaddr_normalize(
        &attr
            .to_str()
            .ok_or("Failed to convert OsStr to String")?
            .to_string(),
    )?;

    Ok(addr)
}

pub fn get_prefix_from_file(path: &str) -> Result<String, Box<dyn Error>> {
    let mut f = File::open(path)?;
    let mut content = String::new();

    f.read_to_string(&mut content)?;

    let re = Regex::new(r"net.ifnames.prefix=([[:alpha:]]+)")?;
    let prefix = match re.captures(&content) {
        Some(c) => c[1].to_string(),
        None => "".to_string(),
    };

    Ok(prefix)
}

pub fn prefix_ok<T: AsRef<str>>(prefix: &T) -> bool {
    // List of forbidden prefixes include kernel's default prefix (eth), biosdevname's prefix (em)
    // and several other prefixes used by udev's net_id built-in
    // https://github.com/systemd/systemd/blob/master/src/udev/udev-builtin-net_id.c#L20
    let forbidden = vec![
        "eth", "eno", "ens", "enb", "enc", "enx", "enP", "enp", "env", "ena", "em",
    ];

    !forbidden.iter().any(|&p| p == prefix.as_ref()) && prefix.as_ref().len() < 16
}

pub fn exit_maybe_unlock(sema: Option<&mut Semaphore>, exit_code: i32) -> ! {
    if let Some(s) = sema {
        s.unlock();
    }

    std::process::exit(exit_code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hwaddr_valid_ok() {
        assert!(hwaddr_valid(&"11:22:33:44:55:66"));
    }

    #[test]
    fn hwaddr_valid_ok_dashed() {
        assert!(hwaddr_valid(&"11-22-33-44-55-66"));
    }

    #[test]
    #[should_panic]
    fn hwaddr_valid_invalid_chars() {
        assert!(hwaddr_valid(&"11-22-33-44-55-xx"));
    }

    #[test]
    #[should_panic]
    fn hwaddr_valid_invalid_range() {
        assert!(hwaddr_valid(&"ffff-33-44-55-66"));
    }

    #[test]
    #[should_panic]
    fn hwaddr_valid_invalid_long() {
        assert!(hwaddr_valid(&"11-22-33-44-55-66-77"));
    }

    #[test]
    #[should_panic]
    fn hwaddr_valid_invalid_short() {
        assert!(hwaddr_valid(&"52:54:00:52:1f"));
    }

    #[test]
    fn hwaddr_normalize_ok() {
        assert_eq!(
            hwaddr_normalize(&"52:54:00:52:1f:93").unwrap(),
            "52:54:00:52:1F:93"
        );
    }

    #[test]
    fn hwaddr_normalize_ok_dashed() {
        assert_eq!(
            hwaddr_normalize(&"52-54-00-52-1f-93").unwrap(),
            "52:54:00:52:1F:93"
        );
    }

    #[test]
    #[should_panic]
    fn hwaddr_normalize_invalid() {
        assert_eq!(
            hwaddr_normalize(&"xx:54:00:52:1f:93").unwrap(),
            "52:54:00:52:1F:93"
        );
    }

    #[test]
    fn net_prefix_ok() {
        assert_eq!(true, prefix_ok(&"net"));
    }

    #[test]
    fn eth_prefix_not_ok() {
        assert_eq!(false, prefix_ok(&"eth"));
    }

    #[test]
    fn long_prefix_not_ok() {
        assert_eq!(false, prefix_ok(&"neeeeeeeeeeeeeeet"));
    }

    #[test]
    fn rename_is_needed() {
        assert_eq!(rename_needed("eth0", "net").unwrap(), true);
    }

    #[test]
    fn rename_not_needed() {
        assert_eq!(rename_needed("net0", "net").unwrap(), false);
    }

    #[test]
    fn rename_needed_interface_unset() {
        assert_eq!(rename_needed("", "net").unwrap(), true);
    }

    #[test]
    fn event_device_not_virtual() {
        env::set_var(
            "DEVPATH",
            "/devices/pci0000:00/0000:00:03.0/virtio0/net/eth0",
        );

        assert_eq!(event_device_virtual(), false);
    }

    #[test]
    fn event_device_is_virtual() {
        env::set_var("DEVPATH", "/devices/virtual/net/bond0");

        assert_eq!(event_device_virtual(), true);
    }
}
