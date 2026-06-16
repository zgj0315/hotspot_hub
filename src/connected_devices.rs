use std::collections::HashSet;

pub struct NeighborEntry<'a> {
    pub interface_name: &'a str,
    pub state: u16,
    pub link_address: &'a [u8],
}

pub fn estimate_count_from_arp(arp_table: &str) -> Option<u32> {
    let mut saw_hotspot_interface = false;
    let mut clients = HashSet::new();

    for line in arp_table.lines().skip(1) {
        let columns: Vec<&str> = line.split_whitespace().collect();
        if columns.len() < 6 {
            continue;
        }

        let flags = columns[2];
        let mac = columns[3].to_ascii_lowercase();
        let device = columns[5];

        if !is_hotspot_interface(device) {
            continue;
        }

        saw_hotspot_interface = true;
        if is_complete_arp_entry(flags) && is_usable_mac(&mac) {
            clients.insert(mac);
        }
    }

    saw_hotspot_interface.then_some(clients.len() as u32)
}

pub fn estimate_count_from_neighbors(entries: &[NeighborEntry<'_>]) -> Option<u32> {
    let mut saw_hotspot_interface = false;
    let mut clients = HashSet::new();

    for entry in entries {
        add_neighbor_entry(
            entry.interface_name,
            entry.state,
            entry.link_address,
            &mut saw_hotspot_interface,
            &mut clients,
        );
    }

    saw_hotspot_interface.then_some(clients.len() as u32)
}

pub fn estimate_count_from_dumpsys_wifi(wifi_dump: &str) -> Option<u32> {
    wifi_dump.lines().find_map(|line| {
        let (_, count) = line.split_once("mNumAssociatedStations:")?;
        count.trim().parse::<u32>().ok()
    })
}

pub fn estimate_count_from_ip_neigh(output: &str) -> Option<u32> {
    let mut saw_hotspot_interface = false;
    let mut clients = HashSet::new();

    for line in output.lines() {
        let columns: Vec<&str> = line.split_whitespace().collect();
        let Some(address) = columns.first() else {
            continue;
        };
        let Some(interface_name) = value_after(&columns, "dev") else {
            continue;
        };

        if !is_hotspot_interface(interface_name) {
            continue;
        }

        saw_hotspot_interface = true;
        if !is_ipv4_address(address) {
            continue;
        }
        if !columns
            .iter()
            .any(|column| is_active_neighbor_state_name(column))
        {
            continue;
        }

        let Some(mac) = value_after(&columns, "lladdr") else {
            continue;
        };
        let mac = mac.to_ascii_lowercase();
        if is_usable_mac(&mac) {
            clients.insert(mac);
        }
    }

    saw_hotspot_interface.then_some(clients.len() as u32)
}

fn add_neighbor_entry(
    interface_name: &str,
    state: u16,
    link_address: &[u8],
    saw_hotspot_interface: &mut bool,
    clients: &mut HashSet<Vec<u8>>,
) {
    if !is_hotspot_interface(interface_name) {
        return;
    }

    *saw_hotspot_interface = true;
    if is_active_neighbor_state(state)
        && link_address.len() == 6
        && !is_zero_link_address(link_address)
    {
        clients.insert(link_address.to_vec());
    }
}

fn is_complete_arp_entry(flags: &str) -> bool {
    let value = flags
        .strip_prefix("0x")
        .and_then(|hex| u32::from_str_radix(hex, 16).ok())
        .or_else(|| flags.parse::<u32>().ok())
        .unwrap_or_default();
    value & 0x2 != 0
}

fn is_usable_mac(mac: &str) -> bool {
    mac != "00:00:00:00:00:00" && mac != "<incomplete>"
}

fn is_zero_link_address(link_address: &[u8]) -> bool {
    link_address.iter().all(|byte| *byte == 0)
}

fn is_active_neighbor_state(state: u16) -> bool {
    const NUD_INCOMPLETE: u16 = 0x01;
    const NUD_REACHABLE: u16 = 0x02;
    const NUD_STALE: u16 = 0x04;
    const NUD_DELAY: u16 = 0x08;
    const NUD_PROBE: u16 = 0x10;
    const NUD_FAILED: u16 = 0x20;
    const NUD_PERMANENT: u16 = 0x80;

    state & (NUD_INCOMPLETE | NUD_FAILED) == 0
        && state & (NUD_REACHABLE | NUD_STALE | NUD_DELAY | NUD_PROBE | NUD_PERMANENT) != 0
}

fn is_active_neighbor_state_name(state: &str) -> bool {
    matches!(
        state,
        "REACHABLE" | "STALE" | "DELAY" | "PROBE" | "PERMANENT"
    )
}

fn is_ipv4_address(address: &str) -> bool {
    let mut parts = address.split('.');
    let Some(first) = parts.next() else {
        return false;
    };
    let Some(second) = parts.next() else {
        return false;
    };
    let Some(third) = parts.next() else {
        return false;
    };
    let Some(fourth) = parts.next() else {
        return false;
    };

    parts.next().is_none()
        && [first, second, third, fourth]
            .iter()
            .all(|part| part.parse::<u8>().is_ok())
}

fn is_hotspot_interface(device: &str) -> bool {
    let normalized = device.to_ascii_lowercase();
    normalized.starts_with("ap")
        || normalized.starts_with("softap")
        || normalized.starts_with("swlan")
        || normalized == "wlan1"
        || normalized.starts_with("wlan1_")
}

fn value_after<'a>(columns: &'a [&str], key: &str) -> Option<&'a str> {
    columns
        .windows(2)
        .find(|window| window[0] == key)
        .map(|window| window[1])
}

#[cfg(target_os = "android")]
pub fn estimate_count_from_netlink() -> Option<u32> {
    android_netlink::estimate_count()
}

#[cfg(target_os = "android")]
mod android_netlink {
    use super::{add_neighbor_entry, is_hotspot_interface};
    use std::collections::HashSet;
    use std::ffi::CStr;
    use std::mem::size_of;
    use std::os::raw::c_void;
    use std::slice;

    const NLMSG_ERROR: u16 = 2;
    const NLMSG_DONE: u16 = 3;
    const RTM_NEWNEIGH: u16 = 28;
    const RTM_GETNEIGH: u16 = 30;
    const NLM_F_REQUEST: u16 = 0x01;
    const NLM_F_DUMP: u16 = 0x300;
    const NDA_LLADDR: u16 = 2;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct NlMsgHeader {
        len: u32,
        kind: u16,
        flags: u16,
        seq: u32,
        pid: u32,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct NeighborMessage {
        family: u8,
        pad1: u8,
        pad2: u16,
        ifindex: i32,
        state: u16,
        flags: u8,
        kind: u8,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct RouteAttr {
        len: u16,
        kind: u16,
    }

    struct SocketFd(i32);

    impl Drop for SocketFd {
        fn drop(&mut self) {
            unsafe {
                libc::close(self.0);
            }
        }
    }

    pub fn estimate_count() -> Option<u32> {
        let fd = unsafe {
            libc::socket(
                libc::AF_NETLINK,
                libc::SOCK_RAW | libc::SOCK_CLOEXEC,
                libc::NETLINK_ROUTE,
            )
        };
        if fd < 0 {
            log::warn!(
                "connected count netlink socket failed: {}",
                std::io::Error::last_os_error()
            );
            return None;
        }
        let fd = SocketFd(fd);

        send_neighbor_dump_request(fd.0)?;
        read_neighbor_dump(fd.0)
    }

    fn send_neighbor_dump_request(fd: i32) -> Option<()> {
        let header = NlMsgHeader {
            len: (size_of::<NlMsgHeader>() + size_of::<NeighborMessage>()) as u32,
            kind: RTM_GETNEIGH,
            flags: NLM_F_REQUEST | NLM_F_DUMP,
            seq: 1,
            pid: 0,
        };
        let message = NeighborMessage {
            family: libc::AF_UNSPEC as u8,
            pad1: 0,
            pad2: 0,
            ifindex: 0,
            state: 0,
            flags: 0,
            kind: 0,
        };

        let mut request = Vec::with_capacity(header.len as usize);
        request.extend_from_slice(as_bytes(&header));
        request.extend_from_slice(as_bytes(&message));

        let mut addr: libc::sockaddr_nl = unsafe { std::mem::zeroed() };
        addr.nl_family = libc::AF_NETLINK as libc::sa_family_t;
        addr.nl_pid = 0;
        addr.nl_groups = 0;

        let sent = unsafe {
            libc::sendto(
                fd,
                request.as_ptr().cast::<c_void>(),
                request.len(),
                0,
                (&addr as *const libc::sockaddr_nl).cast::<libc::sockaddr>(),
                size_of::<libc::sockaddr_nl>() as libc::socklen_t,
            )
        };

        if sent != request.len() as isize {
            log::warn!(
                "connected count netlink send failed: sent={sent}, expected={}, error={}",
                request.len(),
                std::io::Error::last_os_error()
            );
            return None;
        }

        Some(())
    }

    fn read_neighbor_dump(fd: i32) -> Option<u32> {
        let mut saw_hotspot_interface = false;
        let mut clients = HashSet::new();
        let mut messages = 0_u32;
        let mut buffer = vec![0_u8; 16 * 1024];

        loop {
            let received =
                unsafe { libc::recv(fd, buffer.as_mut_ptr().cast::<c_void>(), buffer.len(), 0) };
            if received <= 0 {
                log::warn!(
                    "connected count netlink recv failed: received={received}, error={}",
                    std::io::Error::last_os_error()
                );
                return None;
            }

            let mut offset = 0_usize;
            let received = received as usize;
            while offset + size_of::<NlMsgHeader>() <= received {
                let header = read_unaligned::<NlMsgHeader>(&buffer[offset..])?;
                if header.len < size_of::<NlMsgHeader>() as u32 {
                    return None;
                }

                let message_len = header.len as usize;
                let message_end = offset.checked_add(message_len)?;
                if message_end > received {
                    return None;
                }

                match header.kind {
                    NLMSG_DONE => {
                        log::info!(
                            "connected count netlink done: messages={messages}, saw_hotspot_interface={saw_hotspot_interface}, clients={}",
                            clients.len()
                        );
                        return saw_hotspot_interface.then_some(clients.len() as u32);
                    }
                    NLMSG_ERROR => {
                        log::warn!("connected count netlink returned NLMSG_ERROR");
                        return None;
                    }
                    RTM_NEWNEIGH => {
                        messages += 1;
                        parse_neighbor(
                            &buffer[offset + size_of::<NlMsgHeader>()..message_end],
                            &mut saw_hotspot_interface,
                            &mut clients,
                        )?
                    }
                    _ => {}
                }

                offset = offset.checked_add(align_to_4(message_len))?;
            }
        }
    }

    fn parse_neighbor(
        payload: &[u8],
        saw_hotspot_interface: &mut bool,
        clients: &mut HashSet<Vec<u8>>,
    ) -> Option<()> {
        if payload.len() < size_of::<NeighborMessage>() {
            return Some(());
        }

        let neighbor = read_unaligned::<NeighborMessage>(payload)?;
        let Some(interface_name) = interface_name(neighbor.ifindex) else {
            return Some(());
        };
        let mut link_address = None;
        let mut offset = align_to_4(size_of::<NeighborMessage>());

        while offset + size_of::<RouteAttr>() <= payload.len() {
            let attr = read_unaligned::<RouteAttr>(&payload[offset..])?;
            if attr.len < size_of::<RouteAttr>() as u16 {
                break;
            }

            let attr_len = attr.len as usize;
            let data_start = offset + size_of::<RouteAttr>();
            let data_end = offset.checked_add(attr_len)?;
            if data_end > payload.len() {
                break;
            }

            if attr.kind == NDA_LLADDR {
                link_address = Some(&payload[data_start..data_end]);
            }

            offset = offset.checked_add(align_to_4(attr_len))?;
        }

        if let Some(link_address) = link_address {
            if is_hotspot_interface(&interface_name) {
                log::info!(
                    "connected count netlink hotspot neighbor: iface={interface_name}, state=0x{:x}, link_len={}",
                    neighbor.state,
                    link_address.len()
                );
            }
            add_neighbor_entry(
                &interface_name,
                neighbor.state,
                link_address,
                saw_hotspot_interface,
                clients,
            );
        }

        Some(())
    }

    fn interface_name(ifindex: i32) -> Option<String> {
        let mut name = [0 as libc::c_char; libc::IF_NAMESIZE];
        let ptr = unsafe { libc::if_indextoname(ifindex as libc::c_uint, name.as_mut_ptr()) };
        if ptr.is_null() {
            return None;
        }

        Some(
            unsafe { CStr::from_ptr(ptr) }
                .to_string_lossy()
                .into_owned(),
        )
    }

    fn align_to_4(value: usize) -> usize {
        (value + 3) & !3
    }

    fn as_bytes<T>(value: &T) -> &[u8] {
        unsafe { slice::from_raw_parts((value as *const T).cast::<u8>(), size_of::<T>()) }
    }

    fn read_unaligned<T: Copy>(bytes: &[u8]) -> Option<T> {
        if bytes.len() < size_of::<T>() {
            return None;
        }
        Some(unsafe { (bytes.as_ptr().cast::<T>()).read_unaligned() })
    }
}
