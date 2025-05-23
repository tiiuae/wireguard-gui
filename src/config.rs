/*
    Copyright 2025 TII (SSRC) and the contributors
    SPDX-License-Identifier: Apache-2.0
*/
use crate::cli::{get_config_file_owner, get_config_file_owner_group};
use crate::utils;
use log::debug;
use log::{error, info};
use nix::unistd::{chown, Gid, Group, Uid, User};
use std::fs;
use std::io;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

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
            if let Some(section) = l.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                Ok(LineType::Section(section.trim().into()))
            } else if let Some((left, right)) = l.split_once('=') {
                Ok(LineType::Attribute(left.trim().into(), right.trim().into()))
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
                i => return Err(format!("Unexpected interface name {i}.")),
            },
            LineType::Attribute(key, value) => {
                if is_in_interface {
                    match key.as_str() {
                        "# Name" => cfg.interface.name = Some(value),
                        "Address" => {
                            if !utils::is_ip_valid(Some(&value)) {
                                return Err(format!("Invalid IP address {value}."));
                            }

                            cfg.interface.address = Some(value);
                        }
                        "ListenPort" => cfg.interface.listen_port = Some(value),
                        "PrivateKey" => {
                            cfg.interface.public_key =
                                match utils::generate_public_key(value.clone()) {
                                    Ok(key) => Some(key),
                                    Err(e) => {
                                        return Err(format!("Generating public key: {e}."));
                                    }
                                };
                            cfg.interface.private_key = Some(value);
                        }
                        "DNS" => cfg.interface.dns = Some(value),
                        "Table" => cfg.interface.table = Some(value),
                        "MTU" => cfg.interface.mtu = Some(value),
                        "PreUp" => cfg.interface.pre_up = Some(value),
                        "PostUp" => cfg.interface.post_up = Some(value),
                        "PreDown" => cfg.interface.pre_down = Some(value),
                        "PostDown" => cfg.interface.post_down = Some(value),
                        k => return Err(format!("Unexpected Interface configuration key {k}.")),
                    }
                } else if is_in_peer {
                    match key.as_str() {
                        "# Name" => tmp_peer.name = Some(value),
                        "AllowedIPs" => tmp_peer.allowed_ips = Some(value),
                        "Endpoint" => tmp_peer.endpoint = Some(value),
                        "PublicKey" => tmp_peer.public_key = Some(value),
                        "PersistentKeepalive" => tmp_peer.persistent_keepalive = Some(value),
                        k => return Err(format!("Unexpected Peer configuration key {k}.")),
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
                    return Err(format!("Unexpected attribute {key}."));
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

fn get_uid_gid(user: &str, group: &str) -> io::Result<(Uid, Gid)> {
    let uid = User::from_name(user)
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "Failed to resolve user"))?
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "User not found"))?
        .uid
        .as_raw();

    let gid = Group::from_name(group)
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "Failed to resolve group"))?
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Group not found"))?
        .gid
        .as_raw();

    Ok((uid.into(), gid.into()))
}

pub fn write_configs_to_path(cfgs: &[&WireguardConfig], path: &Path) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    let mut perms = file.metadata()?.permissions();
    perms.set_mode(0o600);
    file.set_permissions(perms)?; // Iterate through each configuration and write it to a file

    for cfg in cfgs.iter() {
        // Generate the content for the configuration
        let content = crate::config::write_config(cfg);

        // Print content for debugging
        debug!("Writing to file: {:?}", file);
        debug!("Content:\n{}", content);

        // Write the content to the file
        file.write_all(content.as_bytes())?;
    }

    // Resolve UID and GID for the user and group
    match get_uid_gid(get_config_file_owner(), get_config_file_owner_group()) {
        Ok((uid, gid)) => {
            info!("Resolved UID: {}, GID: {}", uid, gid);
            // Now you can proceed with ownership changes or other tasks
            // For example, use nix::unistd::chown(path, Some(uid), Some(gid)) to apply the ownership
            chown(path, Some(uid), Some(gid)).map_err(|_| {
                io::Error::new(io::ErrorKind::Other, "Failed to change file ownership")
            })?;
        }
        Err(err) => {
            error!("Error: {}", err);
            fs::remove_file(path)?;
            return Err(err);
        }
    }

    Ok(())
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
