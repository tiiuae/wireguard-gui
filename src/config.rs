/*
    Copyright 2025 TII (SSRC) and the contributors
    SPDX-License-Identifier: Apache-2.0
*/
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::utils;
use tar::{Builder, EntryType, Header, HeaderMode};
/// Defines the VPN settings for the local node.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Default, Debug)]
pub struct Interface {
    pub name: Option<String>,
    pub address: Option<String>,
    pub listen_port: Option<String>,
    pub private_key: Option<String>,
    pub public_key: Option<String>,
    pub dns: Option<String>,
    pub table: Option<String>,
    pub mtu: Option<String>,
    pub pre_up: Option<String>,
    pub post_up: Option<String>,
    pub pre_down: Option<String>,
    pub post_down: Option<String>,
}

/// Defines the VPN settings for a remote peer capable of routing
/// traffic for one or more addresses (itself and/or other
/// peers). Peers can be either a public bounce server that relays
/// traffic to other peers, or a directly accessible client via
/// LAN/internet that is not behind a NAT and only routes traffic for
/// itself.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Default, Debug)]
pub struct Peer {
    pub name: Option<String>,
    pub allowed_ips: Option<String>,
    pub endpoint: Option<String>,
    pub public_key: Option<String>,
    pub persistent_keepalive: Option<String>,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Default, Debug)]
pub struct WireguardConfig {
    pub interface: Interface,
    pub peers: Vec<Peer>,
}

pub fn parse_config(s: &str) -> Result<WireguardConfig, String> {
    enum LineType {
        Section(String),
        Attribute(String, String),
    }

    let lexed_lines = s // remove_comments(s)
        .split('\n')
        .map(str::trim)
        .enumerate()
        .filter(|(_, l)| !l.is_empty())
        .map(|(i, l)| {
            if l.starts_with('[') && l.ends_with(']') {
                Ok(LineType::Section(l[1..l.len() - 1].trim().into()))
            } else if let Some(pos) = l.chars().position(|c| c == '=') {
                Ok(LineType::Attribute(
                    l[0..pos].trim().into(),
                    l[pos + 1..l.len()].trim().into(),
                ))
            } else {
                Err(format!("Couldn't parse line {}: `{}`", i + 1, l.trim()))
            }
        })
        .collect::<Result<Vec<LineType>, String>>()?;

    let mut cfg = WireguardConfig::default();

    let mut it = lexed_lines.into_iter().peekable();

    // We can be either in interface section or in peer section
    let mut is_in_interface = false;
    let mut is_in_peer = false;

    let mut tmp_peer = Peer::default();

    while let Some(l) = it.next() {
        match l {
            LineType::Section(s) => match s.as_str() {
                "Interface" => {
                    is_in_interface = true;
                    is_in_peer = false;
                }
                "Peer" => {
                    is_in_interface = false;
                    is_in_peer = true;
                }
                i => return Err(format!("Unexpected interface name {}.", i)),
            },
            LineType::Attribute(key, value) => {
                if is_in_interface {
                    match key.as_str() {
                        "# Name" => cfg.interface.name = Some(value),
                        "Address" => cfg.interface.address = Some(value),
                        "ListenPort" => cfg.interface.listen_port = Some(value),
                        "PrivateKey" => {
                            //TODO: this should be removed in next release
                            cfg.interface.public_key =
                                Some(utils::generate_public_key(value.clone()).unwrap());
                            cfg.interface.private_key = Some(value)
                        }
                        "DNS" => cfg.interface.dns = Some(value),
                        "Table" => cfg.interface.table = Some(value),
                        "MTU" => cfg.interface.mtu = Some(value),
                        "PreUp" => cfg.interface.pre_up = Some(value),
                        "PostUp" => cfg.interface.post_up = Some(value),
                        "PreDown" => cfg.interface.pre_down = Some(value),
                        "PostDown" => cfg.interface.post_down = Some(value),
                        k => return Err(format!("Unexpected Interface configuration key {}.", k)),
                    }
                } else if is_in_peer {
                    match key.as_str() {
                        "# Name" => tmp_peer.name = Some(value),
                        "AllowedIPs" => tmp_peer.allowed_ips = Some(value),
                        "Endpoint" => tmp_peer.endpoint = Some(value),
                        "PublicKey" => tmp_peer.public_key = Some(value),
                        "PersistentKeepalive" => tmp_peer.persistent_keepalive = Some(value),
                        k => return Err(format!("Unexpected Peer configuration key {}.", k)),
                    };

                    match it.peek() {
                        Some(LineType::Section(_)) => {
                            cfg.peers.push(tmp_peer.clone());
                            tmp_peer = Peer::default();
                        }
                        None => {
                            cfg.peers.push(tmp_peer.clone());
                        }
                        _ => (),
                    }
                } else {
                    return Err(format!("Unexpected attribute {}.", key));
                }
            }
        }
    }

    Ok(cfg)
}

pub fn write_config(c: &WireguardConfig) -> String {
    let mut res = String::from("[Interface]\n");

    let kvs = [
        c.interface.name.clone().map(|v| ("# Name", v)),
        c.interface.address.clone().map(|v| ("Address", v)),
        c.interface.listen_port.clone().map(|v| ("ListenPort", v)),
        c.interface.private_key.clone().map(|v| ("PrivateKey", v)),
        c.interface.dns.clone().map(|v| ("DNS", v)),
        c.interface.table.clone().map(|v| ("Table", v)),
        c.interface.mtu.clone().map(|v| ("MTU", v)),
        c.interface.pre_up.clone().map(|v| ("PreUp", v)),
        c.interface.post_up.clone().map(|v| ("PostUp", v)),
        c.interface.pre_down.clone().map(|v| ("PreDown", v)),
        c.interface.post_down.clone().map(|v| ("PostDown", v)),
    ];

    for (key, value) in kvs.into_iter().flatten() {
        res.push_str(key);
        res.push_str(" = ");
        res.push_str(value.as_str());
        res.push('\n');
    }
    res.push('\n');

    for peer in c.peers.iter() {
        res.push_str("[Peer]\n");

        let kvs = [
            peer.name.clone().map(|v| ("# Name", v)),
            peer.allowed_ips.clone().map(|v| ("AllowedIPs", v)),
            peer.endpoint.clone().map(|v| ("Endpoint", v)),
            peer.public_key.clone().map(|v| ("PublicKey", v)),
            peer.persistent_keepalive
                .clone()
                .map(|v| ("PersistentKeepalive", v)),
        ];

        for (key, value) in kvs.into_iter().flatten() {
            res.push_str(key);
            res.push_str(" = ");
            res.push_str(value.as_str());
            res.push('\n');
        }
        res.push('\n');
    }

    res
}

pub fn write_configs_to_path(cfgs: Vec<WireguardConfig>, path: PathBuf) -> io::Result<()> {
    let time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

    let file = fs::File::create(path)?;
    let mut ar = Builder::new(file);
    ar.mode(HeaderMode::Complete);
    let mut header = Header::new_gnu();

    for (i, cfg) in cfgs.iter().enumerate() {
        let mut name = cfg
            .interface
            .name
            .clone()
            .unwrap_or_else(|| format!("configuration-{i}"));
        name.push_str(".conf");
        let content = crate::config::write_config(cfg);

        header.set_size(content.as_bytes().len().try_into().unwrap());
        header.set_entry_type(EntryType::Regular);
        header.set_mtime(time.as_secs());
        header.set_mode(0o755);
        header.set_cksum();
        ar.append_data(&mut header, name, content.as_bytes())?;
    }

    ar.finish()
}

pub fn get_value(f: &Option<String>) -> &str {
    match f {
        Some(v) => v,
        None => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    //     #[test]
    //     fn parse_write() {
    //         const CONFIG: &str = "[Interface]
    // # Name = node1.example.tld
    // Address = 192.0.2.3/32
    // ListenPort = 51820
    // PrivateKey = localPrivateKeyAbcAbcAbc=
    // DNS = 1.1.1.1,8.8.8.8
    // Table = 12345
    // MTU = 1500
    // PreUp = /bin/example arg1 arg2 %i
    // PostUp = /bin/example arg1 arg2 %i
    // PreDown = /bin/example arg1 arg2 %i
    // PostDown = /bin/example arg1 arg2 %i

    // [Peer]
    // # Name = node2-node.example.tld
    // AllowedIPs = 192.0.2.1/24
    // Endpoint = node1.example.tld:51820
    // PublicKey = remotePublicKeyAbcAbcAbc=
    // PersistentKeepalive = 25

    // [Peer]
    // # Name = node3-node.example.tld
    // AllowedIPs = 192.0.2.2/24
    // Endpoint = node1.example.tld:51821
    // PublicKey = remotePublicKeyBcdBcdBcd=
    // PersistentKeepalive = 26

    // ";
    //         let cfg = parse_config(CONFIG).unwrap();
    //         let s = write_config(&cfg);
    //         assert_eq!(s, CONFIG);
    //     }
}
