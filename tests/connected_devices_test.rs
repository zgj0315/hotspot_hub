use hotspot_hub::connected_devices::{
    estimate_count_from_arp, estimate_count_from_dumpsys_wifi, estimate_count_from_ip_neigh,
    estimate_count_from_neighbors, NeighborEntry,
};

#[test]
fn estimates_unique_hotspot_clients_from_arp_table() {
    let arp = "\
IP address       HW type     Flags       HW address            Mask     Device
192.168.43.74    0x1         0x2         92:42:54:12:08:08     *        wlan1
192.168.43.75    0x1         0x2         92:42:54:12:08:08     *        wlan1
192.168.43.85    0x1         0x0         8a:d4:5c:82:d0:49     *        wlan1
192.168.1.1      0x1         0x2         aa:bb:cc:dd:ee:ff     *        wlan0
";

    assert_eq!(estimate_count_from_arp(arp), Some(1));
}

#[test]
fn returns_zero_when_hotspot_interface_is_visible_without_complete_clients() {
    let arp = "\
IP address       HW type     Flags       HW address            Mask     Device
192.168.43.85    0x1         0x0         8a:d4:5c:82:d0:49     *        wlan1
";

    assert_eq!(estimate_count_from_arp(arp), Some(0));
}

#[test]
fn returns_none_when_no_hotspot_interface_is_visible() {
    let arp = "\
IP address       HW type     Flags       HW address            Mask     Device
192.168.1.1      0x1         0x2         aa:bb:cc:dd:ee:ff     *        wlan0
";

    assert_eq!(estimate_count_from_arp(arp), None);
}

#[test]
fn estimates_unique_hotspot_clients_from_neighbor_entries() {
    let entries = [
        NeighborEntry {
            interface_name: "wlan1",
            state: 0x02,
            link_address: &[0x92, 0x42, 0x54, 0x12, 0x08, 0x08],
        },
        NeighborEntry {
            interface_name: "wlan1",
            state: 0x04,
            link_address: &[0x92, 0x42, 0x54, 0x12, 0x08, 0x08],
        },
        NeighborEntry {
            interface_name: "wlan1",
            state: 0x20,
            link_address: &[0x8a, 0xd4, 0x5c, 0x82, 0xd0, 0x49],
        },
        NeighborEntry {
            interface_name: "wlan0",
            state: 0x02,
            link_address: &[0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff],
        },
    ];

    assert_eq!(estimate_count_from_neighbors(&entries), Some(1));
}

#[test]
fn estimates_associated_stations_from_wifi_dumpsys() {
    let dump = r#"
        WifiController:
          mSoftApStateMachine:
            mApInterfaceName: wlan1
            mNumAssociatedStations: 2
    "#;

    assert_eq!(estimate_count_from_dumpsys_wifi(dump), Some(2));
}

#[test]
fn estimates_unique_hotspot_clients_from_ip_neigh() {
    let output = r#"
192.168.43.74 dev wlan1 lladdr 92:42:54:12:08:08 STALE
192.168.43.85 dev wlan1 lladdr 8a:d4:5c:82:d0:49 REACHABLE
192.168.43.85 dev wlan1 lladdr 8a:d4:5c:82:d0:49 DELAY
192.168.43.86 dev wlan1 lladdr 00:00:00:00:00:00 FAILED
100.65.1.1 dev rmnet_data3 lladdr aa:bb:cc:dd:ee:ff STALE
    "#;

    assert_eq!(estimate_count_from_ip_neigh(output), Some(2));
}

#[test]
fn ignores_ipv6_only_stale_hotspot_neighbors_from_ip_neigh() {
    let output = r#"
192.168.43.74 dev wlan1 lladdr 92:42:54:12:08:08 DELAY
192.168.43.85 dev wlan1  FAILED
240e:404:e30:911c:9b9:8b95:3365:9207 dev wlan1 lladdr 92:42:54:12:08:08 STALE
fe80::18a5:8acf:7c7a:cb7a dev wlan1 lladdr 8a:d4:5c:82:d0:49 STALE
fe80::1ccc:ec24:27e3:547c dev wlan1 lladdr 92:42:54:12:08:08 REACHABLE
240e:404:e30:911c:bdb5:790a:ceda:823b dev wlan1  FAILED
    "#;

    assert_eq!(estimate_count_from_ip_neigh(output), Some(1));
}
