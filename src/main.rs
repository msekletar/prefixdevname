// SPDX-License-Identifier:  MIT

#[macro_use]
extern crate log;
extern crate env_logger;
extern crate ini;
extern crate libudev;

#[macro_use]
extern crate lazy_static;
extern crate libc;
extern crate regex;

mod config;
mod sema;
mod util;

use config::*;
use sema::*;
use util::*;

fn main() {
    env_logger::init();

    let prefix = match get_prefix_from_file("/proc/cmdline") {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to obtain prefix value: {}", e);
            exit_maybe_unlock(None, 1)
        }
    };

    if prefix.is_empty() {
        info!("No prefix specified on the kernel command line");
        exit_maybe_unlock(None, 0);
    }

    if !prefix_ok(&prefix) {
        error!("Invalid prefix, prefix can't be well-known prefix used for NIC naming by other tools and must be shorter than 16 characters");
        exit_maybe_unlock(None, 0);
    }

    if event_device_virtual() {
        debug!("Called for virtual network device, ignoring");
        exit_maybe_unlock(None, 0);
    }

    let ifname = event_device_name();

    if !rename_needed(&ifname, &prefix).unwrap() {
        println!("{}", ifname);
        exit_maybe_unlock(None, 0);
    }

    let mut sema = match Semaphore::new_with_name("net-prefix-ifnames") {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to initialize semaphore: {}", e);
            exit_maybe_unlock(None, 1)
        }
    };

    sema.lock();

    let mut config = NetSetupLinkConfig::new_with_prefix(&prefix);
    if let Err(e) = config.load() {
        error!("Failed to load current state of network links: {}", e);
        exit_maybe_unlock(Some(&mut sema), 1);
    }

    let event_device_hwaddr = match hwaddr_from_event_device() {
        Ok(d) => d,
        Err(e) => {
            error!(
                "Failed to determine MAC address for the event device: {}",
                e
            );
            exit_maybe_unlock(Some(&mut sema), 1)
        }
    };

    if let Some(_c) = config.for_hwaddr(&event_device_hwaddr) {
        info!("Found net_setup_link config for the event device, not generating new one");
        exit_maybe_unlock(Some(&mut sema), 0);
    }

    let next_link_name = match config.next_link_name() {
        Ok(n) => n,
        Err(e) => {
            error!("Failed to create new name for the link: {}", e);
            exit_maybe_unlock(Some(&mut sema), 1)
        }
    };

    let link_config = match PrefixedLink::new_with_hwaddr(&next_link_name, &event_device_hwaddr) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to create link config object: {}", e);
            exit_maybe_unlock(Some(&mut sema), 1)
        }
    };
    if let Err(e) = link_config.write_link_file() {
        error!("Failed to write link file for {}: {}", link_config.name, e);
        exit_maybe_unlock(Some(&mut sema), 1);
    }

    debug!(
        "New link file was generated at {}",
        link_config
            .link_file_path()
            .into_os_string()
            .into_string()
            .unwrap()
    );
    debug!("Consider rebuilding initrd image, using \"dracut -f\"");

    println!("{}", next_link_name);

    sema.unlock();
}
