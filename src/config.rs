/*
    Copyright 2025 TII (SSRC) and the contributors
    SPDX-License-Identifier: Apache-2.0
*/
use crate::{cli, utils};
use anyhow::Result;
use log::{debug, error, info, warn};
use nix::unistd::{chown, Gid, Group, Uid, User};
use pnet_datalink::interfaces;
use std::fs;
use std::io;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

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
    pub binding_iface: Option<String>,
    pub routing_script_name: Option<String>,
    pub has_script_bind_iface: bool,
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
#[derive(Clone, Default, Debug)]
pub struct RoutingScripts {
    pub path: PathBuf,
    pub name: String,
    pub content: String, // new field for truncated content
    pub pre_up: Option<String>,
    pub post_up: Option<String>,
    pub pre_down: Option<String>,
    pub post_down: Option<String>,
    pub has_bind_interface: bool,
}

pub fn parse_config(s: &str) -> Result<WireguardConfig, String> {
    enum LineType {
        Section(String),
        Attribute(String, String),
    }

    let lexed_lines = s
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
                        "# BindIface" => {
                            cfg.interface.binding_iface = Some(value)
                            // TODO: binding iface bilgisayarda var mı kontrol et varsa atama yap.
                            // TODO: aynısını binding scripts için de yap
                        }
                        "# RoutingScriptName" => {
                            cfg.interface.routing_script_name = Some(value)
                            // TODO: binding iface bilgisayarda var mı kontrol et varsa atama yap.
                            // TODO: aynısını binding scripts için de yap
                        }
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
    // Conditional BindIface only when has_script_bind_iface == true
    let binding_iface_entry = if c.interface.has_script_bind_iface {
        c.interface
            .binding_iface
            .clone()
            .map(|v| ("# BindIface", v))
    } else {
        None
    };

    let kvs = [
        c.interface.name.clone().map(|v| ("# Name", v)),
        binding_iface_entry,
        c.interface
            .routing_script_name
            .clone()
            .map(|v| ("# RoutingScriptName", v)),
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
        .map_err(|_| io::Error::other("Failed to resolve user"))?
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "User not found"))?
        .uid
        .as_raw();

    let gid = Group::from_name(group)
        .map_err(|_| io::Error::other("Failed to resolve group"))?
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Group not found"))?
        .gid
        .as_raw();

    Ok((uid.into(), gid.into()))
}

pub fn write_configs_to_path(cfgs: &[&WireguardConfig], path: &Path) -> io::Result<()> {
    // Make sure the parent directory exists
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            debug!("Parent directory not found. Creating: {}", parent.display());
            fs::create_dir_all(parent)?;
        }
    }
    let mut file = fs::File::create(path)?;
    let mut perms = file.metadata()?.permissions();
    perms.set_mode(0o600);
    file.set_permissions(perms)?; // Iterate through each configuration and write it to a file

    for cfg in cfgs.iter() {
        // Generate the content for the configuration
        let content = crate::config::write_config(cfg);

        // Print content for debugging
        info!("Writing to file: {:?}", file);
        debug!("Content:\n{}", content);

        // Write the content to the file
        file.write_all(content.as_bytes())?;
    }

    // Resolve UID and GID for the user and group
    match get_uid_gid(
        cli::get_config_file_owner(),
        cli::get_config_file_owner_group(),
    ) {
        Ok((uid, gid)) => {
            info!("Resolved UID: {}, GID: {}", uid, gid);
            // Now you can proceed with ownership changes or other tasks
            // For example, use nix::unistd::chown(path, Some(uid), Some(gid)) to apply the ownership
            chown(path, Some(uid), Some(gid))
                .map_err(|_| io::Error::other("Failed to change file ownership"))?;
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
        _ => "unknown",
    }
}

fn get_script_paths() -> Vec<PathBuf> {
    // Make sure the scripts directory exists
    let scripts_dir = cli::get_scripts_dir();
    if !scripts_dir.exists() {
        debug!(
            "Scripts directory not found. Creating: {}",
            scripts_dir.display()
        );
        if let Err(err) = fs::create_dir_all(&scripts_dir) {
            error!("Failed to create scripts directory: {}", err);
            return Vec::new();
        } else {
            debug!("Created scripts directory successfully.");
        }
    }

    // Read and collect all files in the directory
    match fs::read_dir(&scripts_dir) {
        Ok(entries) => entries
            .filter_map(Result::ok) // keep only successful entries
            .map(|e| e.path()) // convert to PathBuf
            .filter(|p| p.is_file()) // only files
            .collect(),
        Err(err) => {
            warn!("Failed to read scripts directory: {}", err);
            Vec::new()
        }
    }
}

/// Read scripts and extract routing keywords
pub fn extract_scripts_metadata() -> (Vec<RoutingScripts>, Option<String>) {
    const MAX_CONTENT_CHARS: usize = 4096;
    let mut errors = Vec::new();
    let mut scripts = Vec::new();
    let mut seen = std::collections::HashMap::<String, std::path::PathBuf>::new();
    let mut duplicates = std::collections::HashSet::new();

    for path in get_script_paths() {
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Detect duplicate names
        if let Some(prev) = seen.insert(name.clone(), path.clone()) {
            let msg = format!(
                "Duplicate script name detected: \"{}\" (files: {:?} and {:?})",
                name, prev, path
            );
            error!("{}", msg);
            errors.push(msg);
            duplicates.insert(name);
            continue;
        }

        // Skip oversized scripts
        if let Ok(meta) = fs::metadata(&path) {
            if meta.len() as usize > MAX_CONTENT_CHARS {
                let msg = format!("Script {} is too large, skipping", name);
                error!("{}", msg);
                errors.push(msg);
                continue;
            }
        }

        // Read and parse script content
        match fs::read_to_string(&path) {
            Ok(content) => match parse_routing_keywords(&content, &name) {
                Ok((pre_up, post_up, pre_down, post_down, has_bind_interface)) => {
                    scripts.push(RoutingScripts {
                        path,
                        name,
                        content,
                        pre_up,
                        post_up,
                        pre_down,
                        post_down,
                        has_bind_interface,
                    })
                }
                Err(e) => {
                    errors.push(e.clone());
                    error!("{}", e);
                }
            },
            Err(e) => {
                let msg = format!("Failed to read script {}: {}", name, e);
                error!("{}", msg);
                errors.push(msg);
            }
        }
    }

    // Remove scripts whose name appeared more than once
    scripts.retain(|s| !duplicates.contains(&s.name));

    let combined_errors = if errors.is_empty() {
        None
    } else {
        Some(errors.join("\n"))
    };

    (scripts, combined_errors)
}

/// Validate the script content.
/// Returns `Ok(())` if valid, otherwise `Err(String)` describing the issue.
/// Validate that the script contains at least one of the required keywords.
/// Parse the script content and extract routing keywords
fn parse_routing_keywords(
    content: &str,
    script_name: &str,
) -> Result<
    (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        bool,
    ),
    String,
> {
    let mut pre_up: Option<String> = None;
    let mut post_up: Option<String> = None;
    let mut pre_down: Option<String> = None;
    let mut post_down: Option<String> = None;
    let mut has_bind_iface = false;

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let keyword = if line.starts_with("PreUp") {
            "PreUp"
        } else if line.starts_with("PostUp") {
            "PostUp"
        } else if line.starts_with("PreDown") {
            "PreDown"
        } else if line.starts_with("PostDown") {
            "PostDown"
        } else {
            continue;
        };

        let Some((_, cmds_raw)) = line.split_once('=') else {
            return Err(format!(
                "Invalid syntax in script '{}': missing '=' in line '{}'",
                script_name, line
            ));
        };

        let cmds_raw = cmds_raw.trim();

        // Split commands
        let parts: Vec<&str> = cmds_raw
            .split(';')
            .map(|c| c.trim())
            .filter(|c| !c.is_empty())
            .collect();

        if parts.is_empty() {
            return Err(format!(
                "Invalid empty command list for {} in script '{}'",
                keyword, script_name
            ));
        }

        // Validate each command
        for cmd in &parts {
            if !(cmd.starts_with("iptables")
                || cmd.starts_with("ip ")
                || cmd.starts_with("ip6tables"))
            {
                return Err(format!(
                    "Invalid command '{}' for {} in script '{}'. Only iptables/ip/ip6tables allowed.",
                    cmd, keyword, script_name
                ));
            }
            // Check if any command includes %bindIface
            if cmd.contains("%bindIface") {
                has_bind_iface = true;
            }
        }

        // Join back
        let joined = parts.join("; ");

        match keyword {
            "PreUp" => pre_up = Some(joined),
            "PostUp" => post_up = Some(joined),
            "PreDown" => pre_down = Some(joined),
            "PostDown" => post_down = Some(joined),
            _ => {}
        }
    }

    if pre_up.is_none() && post_up.is_none() && pre_down.is_none() && post_down.is_none() {
        return Err(format!(
            "Script '{}' does not contain any routing keywords (PreUp, PostUp, PreDown, PostDown)",
            script_name
        ));
    }

    Ok((pre_up, post_up, pre_down, post_down, has_bind_iface))
}

pub fn get_binding_interfaces() -> Vec<String> {
    const TYPE_ETHERNET: u32 = 1;
    const TYPE_IEEE_802_11: u32 = 801;
    interfaces()
        .into_iter()
        .filter(|iface| {
            if iface.is_loopback() {
                return false;
            }
            let type_path = format!("/sys/class/net/{}/type", iface.name);
            if let Ok(content) = fs::read_to_string(type_path) {
                let if_type: u32 = content.trim().parse().unwrap_or(0);
                if_type == TYPE_ETHERNET || if_type == TYPE_IEEE_802_11 // Ethernet or Wi-Fi
            } else {
                false
            }
        })
        .map(|iface| iface.name)
        .collect()
}

/*
Return Err -> there is a problem and needs_save,err_msg
*/
pub fn validate_assign_routing_script(
    scripts: &[RoutingScripts],
    cfg: &mut WireguardConfig,
) -> anyhow::Result<()> {
    if let Some(script_name) = &cfg.interface.routing_script_name {
        // Check if the script name exists in the provided list
        let matched_script = scripts.iter().find(|s| s.name == *script_name);

        if let Some(script) = matched_script {
            macro_rules! compare_field {
                ($field:ident) => {
                    match (&cfg.interface.$field, &script.$field) {
                        (Some(cfg_val), Some(template)) => {
                            let cfg_norm = if let Some(bind_iface) = &cfg.interface.binding_iface {
                                cfg_val.replace(bind_iface, "%bindIface")
                            } else {
                                cfg_val.clone()
                            };

                            if &cfg_norm != template {
                                return Err(anyhow::anyhow!(
                                    "{} script does not match the expected template.",
                                    stringify!($field)
                                ));
                            }
                        }
                        (None, None) => {}
                        _ => {
                            return Err(anyhow::anyhow!(
                                "{} scripts are not identical",
                                stringify!($field)
                            ))
                        }
                    }
                };
            }

            compare_field!(pre_up);
            compare_field!(post_up);
            compare_field!(pre_down);
            compare_field!(post_down);

            cfg.interface.has_script_bind_iface = script.has_bind_interface;
        } else {
            // Script name not found in the list
            let msg = format!(
                "'{script_name}' is not available or valid.\n\
                 Please select again from the menu."
            );
            error!("{}", msg);
            return Err(anyhow::anyhow!(msg));
        }
    }
    Ok(())
}

pub fn validate_binding_iface(
    binding_ifaces: &[String],
    cfg: &WireguardConfig,
) -> anyhow::Result<()> {
    if let Some(iface) = &cfg.interface.binding_iface {
        if !binding_ifaces.contains(&iface) {
            return Err(anyhow::anyhow!("Invalid binding interface: {iface}"));
        }
    }
    Ok(())
}

pub fn reset_interface_hooks(cfg: &mut WireguardConfig) {
    cfg.interface.binding_iface = None;
    cfg.interface.routing_script_name = None;
    cfg.interface.pre_up = None;
    cfg.interface.pre_down = None;
    cfg.interface.post_up = None;
    cfg.interface.post_down = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    use super::parse_routing_keywords;

    #[test]
    fn parse_single_keyword() {
        let content = r#"
            # Comment line
            PreUp = iptables -A INPUT -i %i -j ACCEPT
        "#;

        let (pre_up, post_up, pre_down, post_down, has_bind_interface) =
            parse_routing_keywords(content, "test").expect("Should parse");

        assert_eq!(pre_up.unwrap(), "iptables -A INPUT -i %i -j ACCEPT");
        assert!(post_up.is_none());
        assert!(pre_down.is_none());
        assert!(post_down.is_none());
    }

    #[test]
    fn parse_multiple_keywords() {
        let content = r#"
            # My scripts
            # Vpn config
            PreUp = iptables -A INPUT -p tcp --dport 22 -j ACCEPT
            PostUp = ip rule add ipproto tcp dport 22 table 1234
            PreDown = iptables -D INPUT -p tcp --dport 22 -j ACCEPT
            PostDown = ip rule delete ipproto tcp dport 22 table 1234
        "#;

        let (pre_up, post_up, pre_down, post_down, has_bind_interface) =
            parse_routing_keywords(content, "test").expect("Should parse");

        assert_eq!(
            pre_up.unwrap(),
            "iptables -A INPUT -p tcp --dport 22 -j ACCEPT"
        );
        assert_eq!(
            post_up.unwrap(),
            "ip rule add ipproto tcp dport 22 table 1234"
        );
        assert_eq!(
            pre_down.unwrap(),
            "iptables -D INPUT -p tcp --dport 22 -j ACCEPT"
        );
        assert_eq!(
            post_down.unwrap(),
            "ip rule delete ipproto tcp dport 22 table 1234"
        );
    }

    #[test]
    fn parse_ignores_comments_and_empty_lines() {
        let content = r#"
            #PreUp = wrong
            # Comment
             
            PostDown = iptables -A ....
            # end
        "#;

        let (pre_up, post_up, pre_down, post_down, has_bind_interface) =
            parse_routing_keywords(content, "test").expect("Should parse");

        assert!(pre_up.is_none());
        assert!(post_up.is_none());
        assert!(pre_down.is_none());
        assert_eq!(post_down.unwrap(), "iptables -A ....");
    }

    #[test]
    fn parse_no_keywords_fails() {
        let content = r#"
            # only comments
            echo hello
            something else
        "#;

        let err = parse_routing_keywords(content, "myscript").unwrap_err();

        assert!(err.contains("myscript"), "Error should include script name");
        assert!(
            err.contains("does not contain any routing keywords"),
            "Error message should indicate missing routing keywords"
        );
    }
    #[test]
    fn multiple_keywords_each_with_multicommands() {
        let content = r#"
            PreUp = iptables -A INPUT -i %i -j ACCEPT; iptables -A OUTPUT -o %i -j ACCEPT
            PostUp = iptables -t nat -A PREROUTING -i %i -j DNAT --to 10.0.0.1; iptables -A FORWARD -i %i -j ACCEPT
            PreDown = iptables -D INPUT -i %i -j ACCEPT; iptables -D OUTPUT -o %i -j ACCEPT
            PostDown = echo done; echo really_done
        "#;

        let result = parse_routing_keywords(content, "multi");

        // Must fail because PostDown contains invalid commands
        assert!(
            result.is_err(),
            "Parser should fail due to invalid commands"
        );

        let err = result.unwrap_err();
        assert!(
            err.contains("PostDown"),
            "Error must mention the invalid keyword"
        );
        assert!(
            err.contains("echo"),
            "Error must mention the invalid command"
        );
    }

    #[test]
    fn parse_multicommand_postdown() {
        let content = r#"
            PostDown = iptables -D FORWARD -i %i -j ACCEPT; iptables -t nat -D POSTROUTING -o eth0 -j MASQUERADE
        "#;

        let (_, _, _, post_down, has_bind_interface) =
            parse_routing_keywords(content, "multi").expect("Should parse");

        assert_eq!(
            post_down.unwrap(),
            "iptables -D FORWARD -i %i -j ACCEPT; iptables -t nat -D POSTROUTING -o eth0 -j MASQUERADE"
        );
    }
    //     #[test]PostDown =
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
