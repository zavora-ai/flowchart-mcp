//! Network (mxgraph.cisco19 + general) stencil entries.

use super::Entry;

pub const ENTRIES: &[Entry] = &[
    Entry { key: "net.router", path: "mxgraph.cisco19.routers.router", desc: "Router" },
    Entry { key: "net.switch", path: "mxgraph.cisco19.switches.layer_3_switch", desc: "Layer 3 switch" },
    Entry { key: "net.switch_l2", path: "mxgraph.cisco19.switches.layer_2_switch", desc: "Layer 2 switch" },
    Entry { key: "net.firewall", path: "mxgraph.cisco19.security.firewall", desc: "Firewall" },
    Entry { key: "net.server", path: "mxgraph.cisco19.servers.standard_host", desc: "Server" },
    Entry { key: "net.load_balancer", path: "mxgraph.cisco19.misc.load_balancer", desc: "Load balancer" },
    Entry { key: "net.cloud", path: "mxgraph.cisco19.misc.cloud", desc: "Cloud" },
    Entry { key: "net.workstation", path: "mxgraph.cisco19.computers_and_peripherals.pc", desc: "Workstation / PC" },
    Entry { key: "net.laptop", path: "mxgraph.cisco19.computers_and_peripherals.laptop", desc: "Laptop" },
    Entry { key: "net.mobile", path: "mxgraph.cisco19.computers_and_peripherals.mobile_phone", desc: "Mobile phone" },
    Entry { key: "net.access_point", path: "mxgraph.cisco19.wireless.access_point", desc: "Wireless access point" },
    Entry { key: "net.wlc", path: "mxgraph.cisco19.wireless.wireless_lan_controller", desc: "WLAN controller" },
    Entry { key: "net.ids", path: "mxgraph.cisco19.security.ids", desc: "IDS" },
    Entry { key: "net.vpn_gateway", path: "mxgraph.cisco19.security.vpn_gateway", desc: "VPN gateway" },
    // General networking shapes (rack library).
    Entry { key: "net.rack", path: "mxgraph.rackGeneral.rackNumbering", desc: "Server rack" },
    Entry { key: "net.storage", path: "mxgraph.networks.storage", desc: "Storage array" },
    Entry { key: "net.database", path: "mxgraph.networks.database", desc: "Database server" },
];
