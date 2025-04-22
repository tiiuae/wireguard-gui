/*
    Copyright 2025 TII (SSRC) and the contributors
    SPDX-License-Identifier: Apache-2.0
*/
use std::{collections::HashMap, convert::TryFrom};

use cidr::IpCidr;

use crate::{config::*, utils};

#[derive(Debug)]
pub struct GenerationSettings {
    listen_port: u16,
    number_of_clients: u8,
    cidr: IpCidr,
    client_allowed_ips: Vec<IpCidr>,
    // Endpoint is represented by domain name and port, but keep just as String for simplicity.
    endpoint: Option<String>,
    // dns: Option<String>,
    post_up_rule: Option<String>,
    post_down_rule: Option<String>,
}

impl TryFrom<HashMap<String, Option<String>>> for GenerationSettings {
    type Error = &'static str;
    fn try_from(map: HashMap<String, Option<String>>) -> Result<Self, Self::Error> {
        let listen_port: u16 = map
            .get("Listen Port")
            .cloned()
            .flatten()
            .ok_or("Listen Port is unspecified")
            .and_then(|s| s.parse::<u16>().map_err(|_| "Could not parse Listen Port"))?;
        let number_of_clients: u8 = map
            .get("Number of Clients")
            .cloned()
            .flatten()
            .ok_or("Number of Clients is unspecified")
            .and_then(|s| {
                s.parse::<u8>()
                    .map_err(|_| "Could not parse Number of Clients")
            })?;
        let cidr: IpCidr = map
            .get("CIDR")
            .cloned()
            .flatten()
            .ok_or("No CIDR specified")
            .and_then(|s| s.parse().map_err(|_| "Could not parse CIDR"))?;
        let client_allowed_ips: Vec<IpCidr> = map
            .get("Client Allowed IPs")
            .cloned()
            .flatten()
            .ok_or("No Client Allowed IPs specified")
            .and_then(|s| {
                s.split(',')
                    .map(|addr| addr.trim().parse::<IpCidr>())
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| "Could not parse one of the Allowed IP addresses")
            })?;
        let endpoint: Option<String> = map.get("Endpoint (Optional)").cloned().flatten();
        // let dns: Option<String> = map.get("DNS (Optional)").cloned().flatten();
        let post_up_rule: Option<String> = map.get("Post-Up rule (Optional)").cloned().flatten();
        let post_down_rule: Option<String> =
            map.get("Post-Down rule (Optional)").cloned().flatten();

        Ok(Self {
            listen_port,
            number_of_clients,
            cidr,
            client_allowed_ips,
            endpoint,
            // dns,
            post_up_rule,
            post_down_rule,
        })
    }
}

impl GenerationSettings {
    // TODO: Error handling
    pub fn generate(&self) -> Vec<WireguardConfig> {
        let mut cfgs = Vec::with_capacity(usize::from(self.number_of_clients) + 1);

        let mut cidr_iter = self.cidr.iter();

        let listen_port = self.listen_port.to_string();

        let host_private_key = utils::generate_private_key().unwrap();
        let host_public_key = utils::generate_public_key(host_private_key.clone()).unwrap();

        let mut host_cfg = WireguardConfig {
            interface: Interface {
                address: Some(cidr_iter.next().unwrap().to_string()),
                listen_port: Some(listen_port.clone()),
                private_key: Some(host_private_key),
                post_up: self.post_up_rule.clone(),
                post_down: self.post_down_rule.clone(),
                ..Default::default()
            },
            peers: vec![],
        };

        for client_cidr in cidr_iter.take(self.number_of_clients.into()) {
            let client_cidr: String = client_cidr.to_string();
            let client_private_key = utils::generate_private_key().unwrap();
            let client_public_key = utils::generate_public_key(client_private_key.clone()).unwrap();

            cfgs.push(WireguardConfig {
                interface: Interface {
                    address: Some(client_cidr.clone()),
                    listen_port: Some(listen_port.clone()),
                    private_key: Some(client_private_key),
                    ..Default::default()
                },
                peers: vec![Peer {
                    allowed_ips: Some(
                        self.client_allowed_ips
                            .iter()
                            .map(|ip| ip.to_string())
                            .collect::<Vec<String>>()
                            .join(", "),
                    ),
                    endpoint: self.endpoint.clone(),
                    public_key: Some(host_public_key.clone()),
                    ..Default::default()
                }],
            });

            host_cfg.peers.push(Peer {
                allowed_ips: Some(client_cidr.clone()),
                public_key: Some(client_public_key),
                ..Default::default()
            });
        }

        cfgs.insert(0, host_cfg);

        cfgs
    }
}
