/*
    Copyright 2025 TII (SSRC) and the contributors
    SPDX-License-Identifier: Apache-2.0
*/
use std::{collections::HashMap, convert::TryFrom};

use ipnetwork::IpNetwork;

use crate::{
    config::{Interface, Peer, WireguardConfig},
    utils,
};

#[derive(Debug)]
pub struct GenerationSettings {
    tunnel_iface_name: String,
    tunnel_iface_ip: IpNetwork,
    listen_port: u16,
    number_of_peers: usize,
    // client_allowed_ips: Vec<IpCidr>,
    // Endpoint is represented by domain name and port, but keep just as String for simplicity.
    // endpoint: Option<String>,
    // dns: Option<String>,
    post_up_rule: Option<String>,
    post_down_rule: Option<String>,
}

impl TryFrom<HashMap<String, Option<String>>> for GenerationSettings {
    type Error = &'static str;
    fn try_from(mut map: HashMap<String, Option<String>>) -> Result<Self, Self::Error> {
        let generate_iptables_rule = |action: &str, interface: &str| -> String {
            format!(
                "iptables {action} FORWARD -i {interface} -j ACCEPT; iptables -t nat {action} POSTROUTING -o ethint0 -j MASQUERADE")
        };

        let tunnel_iface_name = map
            .remove("Tunnel interface name")
            .flatten()
            .ok_or("'Tunnel interface name' is unspecified")?;

        let tunnel_iface_ip = map
            .remove("Tunnel interface ip")
            .flatten()
            .ok_or("'Tunnel interface ip' is unspecified")
            .and_then(|s| {
                s.parse::<IpNetwork>()
                    .map_err(|_| "Could not parse 'Tunnel interface ip'")
            })?;

        let listen_port = map
            .remove("Listen Port [default:51820]")
            .flatten()
            .ok_or("Listen Port is unspecified")
            .and_then(|s| s.parse::<u16>().map_err(|_| "Could not parse Listen Port"))?;
        let number_of_peers = map
            .remove("Number of Peers [default:1]")
            .flatten()
            .ok_or("'Number of Peers' is unspecified")
            .and_then(|s| {
                s.parse::<usize>()
                    .map_err(|_| "Could not parse Number of Peers")
            })?;

        // let client_allowed_ips: Vec<IpCidr> = map
        //     .get("Client Allowed IPs")
        //     .cloned()
        //     .flatten()
        //     .ok_or("No Client Allowed IPs specified")
        //     .and_then(|s| {
        //         s.split(',')
        //             .map(|addr| addr.trim().parse::<IpCidr>())
        //             .collect::<Result<Vec<_>, _>>()
        //             .map_err(|_| "Could not parse one of the Allowed IP addresses")
        //     })?;
        // let endpoint: Option<String> = map.get("Endpoint (Optional)").cloned().flatten();
        // let dns: Option<String> = map.get("DNS (Optional)").cloned().flatten();
        let post_up_rule = map
            .remove("Post-Up rule (Optional)")
            .flatten()
            .or(Some(generate_iptables_rule("-A", &tunnel_iface_name)));
        let post_down_rule = map
            .remove("Post-Down rule (Optional)")
            .flatten()
            .or(Some(generate_iptables_rule("-D", &tunnel_iface_name)));

        Ok(Self {
            tunnel_iface_name,
            tunnel_iface_ip,
            listen_port,
            number_of_peers,
            // client_allowed_ips,
            // endpoint,
            // dns,
            post_up_rule,
            post_down_rule,
        })
    }
}

impl GenerationSettings {
    pub fn generate(&self) -> Result<WireguardConfig, std::io::Error> {
        let listen_port = self.listen_port.to_string();

        let host_private_key = utils::generate_private_key()?;
        let host_public_key = utils::generate_public_key(host_private_key.clone())?;

        let mut host_cfg = WireguardConfig {
            interface: Interface {
                name: Some(self.tunnel_iface_name.clone()),
                address: Some(self.tunnel_iface_ip.clone().to_string()),
                listen_port: Some(listen_port.clone()),
                public_key: Some(host_public_key),
                private_key: Some(host_private_key),
                post_up: self.post_up_rule.clone(),
                post_down: self.post_down_rule.clone(),
                ..Default::default()
            },
            peers: vec![],
        };
        let number_of_peers = self.number_of_peers;

        host_cfg.peers.extend((0..number_of_peers).map(|_| Peer {
            allowed_ips: Some("ip/netmask".to_string()),
            endpoint: Some("<peer public ip>:51820".to_string()),
            public_key: None,
            ..Default::default()
        }));

        Ok(host_cfg)
    }
}
