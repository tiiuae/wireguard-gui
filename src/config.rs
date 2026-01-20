/*
    Copyright 2025 TII (SSRC) and the contributors
    SPDX-License-Identifier: Apache-2.0
*/
use crate::{cli, utils};
use anyhow::{Context, Result, anyhow};
use log::{debug, error, info, warn};
use nix::unistd::{Gid, Group, Uid, User, chown};
use pnet_datalink::interfaces;
use std::fs;
use std::io;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::str::FromStr;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RoutingKeyword {
    PreUp,
    PostUp,
    PreDown,
    PostDown,
    FwMark,
}

impl FromStr for RoutingKeyword {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "PreUp" => Ok(RoutingKeyword::PreUp),
            "PostUp" => Ok(RoutingKeyword::PostUp),
            "PreDown" => Ok(RoutingKeyword::PreDown),
            "PostDown" => Ok(RoutingKeyword::PostDown),
            "FwMark" => Ok(RoutingKeyword::FwMark),
            _ => Err(()),
        }
    }
}

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
    pub fwmark: Option<String>,
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
pub struct RoutingHooks {
    pub pre_up: Option<String>,
    pub post_up: Option<String>,
    pub pre_down: Option<String>,
    pub post_down: Option<String>,
    pub fwmark: Option<String>,
    pub has_bind_interface: bool,
}
#[derive(Clone, Default, Debug)]
pub struct RoutingScripts {
    pub path: PathBuf,
    pub name: String,
    pub content: String, // new field for truncated content
    pub routing_hooks: RoutingHooks,
}

pub fn parse_config(s: &str) -> Result<WireguardConfig, String> {
    enum LineType {
        Section(String),
        Attribute(String, String),
        Comment(String),
    }

    let lexed_lines = s
        .split('\n')
        .map(str::trim)
        .enumerate()
        .filter(|(_, l)| !l.is_empty())
        .map(|(i, l)| {
            if let Some(section) = l.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                Ok(LineType::Section(section.trim().into()))
            } else if let Some(comment) = l.strip_prefix('#') {
                Ok(LineType::Comment(comment.trim().into()))
            } else if let Some((left, right)) = l.split_once('=') {
                Ok(LineType::Attribute(left.trim().into(), right.trim().into()))
            } else {
                Err(format!("Couldn't parse line {}: `{}`", i + 1, l.trim()))
            }
        })
        .collect::<Result<Vec<LineType>, String>>()?;

    let mut cfg = WireguardConfig::default();
    let mut it = lexed_lines.into_iter().peekable();

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
            LineType::Comment(comment) => {
                // Handle comment lines (# key = value or # standalone comment)
                if let Some((key, value)) = comment.split_once('=') {
                    let key = key.trim();
                    let value = value.trim().to_string();

                    if is_in_interface {
                        match key {
                            // Standard WireGuard comment attributes
                            "Name" => cfg.interface.name = Some(value),
                            "BindIface" => cfg.interface.binding_iface = Some(value),
                            "RoutingScriptName" => cfg.interface.routing_script_name = Some(value),
                            // Ignore unknown comments (Proton VPN: Bouncing, NAT-PMP, etc.)
                            _ => {}
                        }
                    } else if is_in_peer && key == "Name" {
                        tmp_peer.name = Some(value);
                    }
                } else if is_in_interface {
                    // Proton VPN: "# Key for <user>" -> name
                    if let Some(user) = comment.strip_prefix("Key for ") {
                        cfg.interface.name = Some(user.to_string());
                    }
                } else if is_in_peer && tmp_peer.name.is_none() {
                    // Proton VPN: First standalone comment after [Peer] is the server name
                    // e.g., "# NL-FREE#241" or "# AT#133"
                    tmp_peer.name = Some(comment.to_string());
                }
            }
            LineType::Attribute(key, value) => {
                if is_in_interface {
                    match key.as_str() {
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
                        "FwMark" => cfg.interface.fwmark = Some(value),
                        k => return Err(format!("Unexpected Interface configuration key: {k}")),
                    }
                } else if is_in_peer {
                    match key.as_str() {
                        "AllowedIPs" => tmp_peer.allowed_ips = Some(value),
                        "Endpoint" => tmp_peer.endpoint = Some(value),
                        "PublicKey" => tmp_peer.public_key = Some(value),
                        "PersistentKeepalive" => tmp_peer.persistent_keepalive = Some(value),
                        k => return Err(format!("Unexpected Peer configuration key: {k}")),
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
    let iface = &c.interface;

    // Interface section
    let iface_kvs = [
        Some("# Name").zip(iface.name.as_deref()),
        Some("# BindIface")
            .zip(iface.binding_iface.as_deref())
            .filter(|_| iface.has_script_bind_iface),
        Some("# RoutingScriptName").zip(iface.routing_script_name.as_deref()),
        Some("Address").zip(iface.address.as_deref()),
        Some("ListenPort").zip(iface.listen_port.as_deref()),
        Some("PrivateKey").zip(iface.private_key.as_deref()),
        Some("DNS").zip(iface.dns.as_deref()),
        Some("Table").zip(iface.table.as_deref()),
        Some("MTU").zip(iface.mtu.as_deref()),
        Some("PreUp").zip(iface.pre_up.as_deref()),
        Some("PostUp").zip(iface.post_up.as_deref()),
        Some("PreDown").zip(iface.pre_down.as_deref()),
        Some("PostDown").zip(iface.post_down.as_deref()),
        Some("FwMark").zip(iface.fwmark.as_deref()),
    ];
    for (key, value) in iface_kvs.into_iter().flatten() {
        res.push_str(key);
        res.push_str(" = ");
        res.push_str(value);
        res.push('\n');
    }
    res.push('\n');

    for peer in &c.peers {
        res.push_str("[Peer]\n");

        let peer_kvs = [
            Some("# Name").zip(peer.name.as_deref()),
            Some("AllowedIPs").zip(peer.allowed_ips.as_deref()),
            Some("Endpoint").zip(peer.endpoint.as_deref()),
            Some("PublicKey").zip(peer.public_key.as_deref()),
            Some("PersistentKeepalive").zip(peer.persistent_keepalive.as_deref()),
        ];

        for (key, value) in peer_kvs.into_iter().flatten() {
            res.push_str(key);
            res.push_str(" = ");
            res.push_str(value);
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

pub fn write_config_to_path(cfg: &WireguardConfig, path: &Path) -> anyhow::Result<()> {
    // Helper to delete the file only if it already exists
    let cleanup = |_: &anyhow::Error| {
        let _ = fs::remove_file(path);
    };
    // 1. Ensure parent dir exists
    let parent = path
        .parent()
        .context("Provided path has no parent directory")?;

    if !parent.exists() {
        debug!("Creating parent directory: {}", parent.display());
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }

    // Create file with secure permissions
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
        .with_context(|| format!("Failed to create file {}", path.display()))?;

    // Generate the content for the configuration
    let content = crate::config::write_config(cfg);

    info!("Writing to file: {}", path.display());
    debug!("Content:\n{}", content);

    //  Write contents — if writing fails, delete the file
    file.write_all(content.as_bytes())
        .with_context(|| format!("Failed to write configuration to {}", path.display()))
        .inspect_err(cleanup)?;

    // Ensure data is flushed to disk
    file.sync_all()
        .with_context(|| format!("Failed to sync file {}", path.display()))
        .inspect_err(cleanup)?;

    // Resolve UID and GID for the user and group
    let (uid, gid) = get_uid_gid(
        cli::get_config_file_owner(),
        cli::get_config_file_owner_group(),
    )
    .context("Failed to resolve UID/GID")
    .inspect_err(cleanup)?;

    // Set file ownership
    chown(path, Some(uid), Some(gid))
        .with_context(|| format!("Failed to change ownership for {}", path.display()))
        .inspect_err(cleanup)?;

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
    const MAX_CONTENT_CHARS: u64 = 4096;
    let mut errors = Vec::new();
    let mut scripts = Vec::new();

    for path in get_script_paths() {
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Skip oversized scripts
        if let Ok(meta) = fs::metadata(&path)
            && meta.len() > MAX_CONTENT_CHARS
        {
            let msg = format!("Script {} is too large, skipping", name);
            error!("{}", msg);
            errors.push(msg);
            continue;
        }

        // Read and parse script content
        match fs::read_to_string(&path) {
            Ok(content) => match parse_routing_keywords(&content, &name) {
                Ok(routing_hooks) => scripts.push(RoutingScripts {
                    path,
                    name,
                    content,
                    routing_hooks,
                }),
                Err(e) => {
                    error!("{}", e);
                    errors.push(e.to_string());
                }
            },
            Err(e) => {
                let msg = format!("Failed to read script {}: {}", name, e);
                error!("{}", msg);
                errors.push(msg);
            }
        }
    }

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
fn parse_routing_keywords(content: &str, script_name: &str) -> Result<RoutingHooks> {
    let mut pre_up: Option<String> = None;
    let mut post_up: Option<String> = None;
    let mut pre_down: Option<String> = None;
    let mut post_down: Option<String> = None;
    let mut fwmark: Option<String> = None;
    let mut has_bind_interface = false;

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Split line into key/value
        let (key, cmds_raw) = line.split_once('=').ok_or_else(|| {
            anyhow!(
                "Invalid syntax in script '{}': missing '=' in line '{}'",
                script_name,
                line
            )
        })?;

        // Convert string to enum
        let keyword = key.trim().parse::<RoutingKeyword>().map_err(|_| {
            anyhow!(
                "Unknown routing keyword '{}' in script '{}'",
                key.trim(),
                script_name
            )
        })?;

        let cmds_raw = cmds_raw.trim();
        let parts: Vec<&str> = cmds_raw
            .split(';')
            .map(|c| c.trim())
            .filter(|c| !c.is_empty())
            .collect();

        if parts.is_empty() {
            anyhow::bail!(
                "Empty command list for {:?} in script '{}'",
                keyword,
                script_name
            );
        }

        if keyword != RoutingKeyword::FwMark {
            for cmd in &parts {
                if !(cmd.starts_with("iptables")
                    || cmd.starts_with("ip ")
                    || cmd.starts_with("ip6tables"))
                {
                    anyhow::bail!(
                        "Invalid command '{}' for {:?} in script '{}'. Only iptables/ip/ip6tables allowed.",
                        cmd,
                        keyword,
                        script_name
                    );
                }
                if cmd.contains("%bindIface") {
                    has_bind_interface = true;
                }
            }
        }

        let joined = parts.join("; ");

        match keyword {
            RoutingKeyword::PreUp => pre_up = Some(joined),
            RoutingKeyword::PostUp => post_up = Some(joined),
            RoutingKeyword::PreDown => pre_down = Some(joined),
            RoutingKeyword::PostDown => post_down = Some(joined),
            RoutingKeyword::FwMark => fwmark = Some(joined),
        }
    }

    if pre_up.is_none() && post_up.is_none() && pre_down.is_none() && post_down.is_none() {
        anyhow::bail!(
            "Script '{}' does not contain any routing keywords (PreUp, PostUp, PreDown, PostDown)",
            script_name
        );
    }

    Ok(RoutingHooks {
        pre_up,
        post_up,
        pre_down,
        post_down,
        fwmark,
        has_bind_interface,
    })
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
            fs::read_to_string(type_path)
            .is_ok_and(|content| matches!(content.trim().parse::<u32>(), Ok(v) if v == TYPE_ETHERNET || v == TYPE_IEEE_802_11))
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
    let script_name = match &cfg.interface.routing_script_name {
        Some(name) => name,
        None => return Ok(()),
    };

    let script = scripts
        .iter()
        .find(|s| &s.name == script_name)
        .ok_or_else(|| {
            anyhow!(
                "'{script_name}' is not available or valid.\nPlease select again from the menu."
            )
        })?;

    let check_field = |cfg_val: &Option<String>,
                       template_val: &Option<String>,
                       use_bind: bool,
                       name: &str|
     -> anyhow::Result<()> {
        match (cfg_val, template_val) {
            (Some(cfg_val), Some(template)) => {
                let normalized = if use_bind {
                    cfg.interface
                        .binding_iface
                        .as_ref()
                        .map(|iface| cfg_val.replace(iface, "%bindIface"))
                        .unwrap_or_else(|| cfg_val.clone())
                } else {
                    cfg_val.clone()
                };
                if normalized != *template {
                    anyhow::bail!("{name} script does not match the expected template.");
                }
            }
            (None, None) => {}
            _ => anyhow::bail!("{name} scripts are not identical"),
        }
        Ok(())
    };

    check_field(
        &cfg.interface.pre_up,
        &script.routing_hooks.pre_up,
        true,
        "pre_up",
    )?;
    check_field(
        &cfg.interface.post_up,
        &script.routing_hooks.post_up,
        true,
        "post_up",
    )?;
    check_field(
        &cfg.interface.pre_down,
        &script.routing_hooks.pre_down,
        true,
        "pre_down",
    )?;
    check_field(
        &cfg.interface.post_down,
        &script.routing_hooks.post_down,
        true,
        "post_down",
    )?;
    check_field(
        &cfg.interface.fwmark,
        &script.routing_hooks.fwmark,
        false,
        "fwmark",
    )?;

    cfg.interface.has_script_bind_iface = script.routing_hooks.has_bind_interface;

    Ok(())
}

pub fn validate_binding_iface(
    binding_ifaces: &[String],
    cfg: &WireguardConfig,
) -> anyhow::Result<()> {
    if let Some(iface) = &cfg.interface.binding_iface
        && !binding_ifaces.contains(iface)
    {
        anyhow::bail!("Invalid binding interface: {iface}");
    }

    Ok(())
}

pub fn reset_interface_hooks(cfg: &mut WireguardConfig) {
    cfg.interface = Interface {
        name: cfg.interface.name.take(),
        address: cfg.interface.address.take(),
        listen_port: cfg.interface.listen_port.take(),
        private_key: cfg.interface.private_key.take(),
        public_key: cfg.interface.public_key.take(),
        dns: cfg.interface.dns.take(),
        table: cfg.interface.table.take(),
        mtu: cfg.interface.mtu.take(),
        binding_iface: None,
        routing_script_name: None,
        pre_up: None,
        pre_down: None,
        post_up: None,
        post_down: None,
        fwmark: None,
        has_script_bind_iface: false,
    };
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

        let routing_hooks = parse_routing_keywords(content, "test").expect("Should parse");

        assert_eq!(
            routing_hooks.pre_up.unwrap(),
            "iptables -A INPUT -i %i -j ACCEPT"
        );
        assert!(routing_hooks.post_up.is_none());
        assert!(routing_hooks.pre_down.is_none());
        assert!(routing_hooks.post_down.is_none());
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

        let routing_hooks = parse_routing_keywords(content, "test").expect("Should parse");

        assert_eq!(
            routing_hooks.pre_up.unwrap(),
            "iptables -A INPUT -p tcp --dport 22 -j ACCEPT"
        );
        assert_eq!(
            routing_hooks.post_up.unwrap(),
            "ip rule add ipproto tcp dport 22 table 1234"
        );
        assert_eq!(
            routing_hooks.pre_down.unwrap(),
            "iptables -D INPUT -p tcp --dport 22 -j ACCEPT"
        );
        assert_eq!(
            routing_hooks.post_down.unwrap(),
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

        let routing_hooks = parse_routing_keywords(content, "test").expect("Should parse");

        assert!(routing_hooks.pre_up.is_none());
        assert!(routing_hooks.post_up.is_none());
        assert!(routing_hooks.pre_down.is_none());
        assert_eq!(routing_hooks.post_down.unwrap(), "iptables -A ....");
    }
    #[test]
    fn parse_unknown_keywords_fails() {
        let content = r#"
            # only comments and unknown keys
            FOO = bar
            BAR = baz
        "#;

        let err = parse_routing_keywords(content, "myscript")
            .unwrap_err()
            .to_string();

        // Check that the error mentions the script name
        assert!(err.contains("myscript"), "Error should include script name");

        // Check that the error indicates missing routing keywords
        assert!(
            err.contains("Unknown routing keyword"),
            "Error message should indicate missing routing keywords"
        );
    }
    #[test]
    fn parse_invalid_syntax_fails() {
        let content = r#"
            # only comments
            echo hello
            something else
        "#;

        let err = parse_routing_keywords(content, "myscript")
            .unwrap_err()
            .to_string();

        // Check that the error mentions the script name
        assert!(err.contains("myscript"), "Error should include script name");

        // Check that the error mentions invalid syntax or missing routing keywords
        assert!(
            err.contains("Invalid syntax"),
            "Error message should indicate invalid syntax or missing routing keywords"
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

        let err = result.unwrap_err().to_string();
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

        let routing_hooks = parse_routing_keywords(content, "multi").expect("Should parse");

        assert_eq!(
            routing_hooks.post_down.unwrap(),
            "iptables -D FORWARD -i %i -j ACCEPT; iptables -t nat -D POSTROUTING -o eth0 -j MASQUERADE"
        );
    }
    fn iface(minimal: bool) -> Interface {
        Interface {
            name: Some("wg0".into()),
            binding_iface: Some("eth0".into()),
            has_script_bind_iface: !minimal,
            routing_script_name: Some("route.sh".into()),
            address: Some("10.0.0.1/24".into()),
            listen_port: Some("51820".into()),
            public_key: Some("pubkey".into()),
            private_key: Some("privkey".into()),
            dns: Some("1.1.1.1".into()),
            table: Some("auto".into()),
            mtu: Some("1420".into()),
            pre_up: Some("foo".into()),
            post_up: Some("bar".into()),
            pre_down: Some("baz".into()),
            post_down: Some("qux".into()),
            fwmark: Some("123".into()),
        }
    }

    fn peer(name: &str) -> Peer {
        Peer {
            name: Some(name.into()),
            allowed_ips: Some("10.0.0.2/32".into()),
            endpoint: Some("peer.example.com:51820".into()),
            public_key: Some("pubkey".into()),
            persistent_keepalive: Some("25".into()),
        }
    }

    #[test]
    fn writes_basic_interface() {
        let cfg = WireguardConfig {
            interface: iface(false),
            peers: vec![],
        };

        let out = write_config(&cfg);

        assert!(out.contains("[Interface]"));
        assert!(out.contains("# Name = wg0"));
        assert!(out.contains("# BindIface = eth0"));
        assert!(out.contains("# RoutingScriptName = route.sh"));
        assert!(out.contains("Address = 10.0.0.1/24"));
        assert!(out.contains("PrivateKey = privkey"));
        assert!(out.contains("FwMark = 123"));
    }

    #[test]
    fn suppresses_binding_iface_when_flag_false() {
        let mut iface = iface(true); // minimal: no bind iface shown
        iface.has_script_bind_iface = false;

        let cfg = WireguardConfig {
            interface: iface,
            peers: vec![],
        };

        let out = write_config(&cfg);

        assert!(!out.contains("# BindIface"));
    }

    #[test]
    fn omits_missing_fields() {
        let iface = Interface {
            name: Some("wg0".into()),
            binding_iface: None,
            has_script_bind_iface: true,
            routing_script_name: None,
            address: None,
            listen_port: None,
            private_key: None,
            public_key: None,
            dns: None,
            table: None,
            mtu: None,
            pre_up: None,
            post_up: None,
            pre_down: None,
            post_down: None,
            fwmark: None,
        };

        let cfg = WireguardConfig {
            interface: iface,
            peers: vec![],
        };

        let out = write_config(&cfg);

        assert!(out.contains("# Name = wg0"));
        assert!(!out.contains("Address ="));
        assert!(!out.contains("# BindIface"));
    }

    #[test]
    fn writes_multiple_peers() {
        let cfg = WireguardConfig {
            interface: iface(false),
            peers: vec![peer("peer1"), peer("peer2")],
        };

        let out = write_config(&cfg);

        let count_peer_headers = out.matches("[Peer]").count();
        assert_eq!(count_peer_headers, 2);

        assert!(out.contains("# Name = peer1"));
        assert!(out.contains("# Name = peer2"));
    }

    #[test]
    fn peer_fields_render_correctly() {
        let cfg = WireguardConfig {
            interface: iface(false),
            peers: vec![peer("p")],
        };

        let out = write_config(&cfg);

        assert!(out.contains("AllowedIPs = 10.0.0.2/32"));
        assert!(out.contains("Endpoint = peer.example.com:51820"));
        assert!(out.contains("PersistentKeepalive = 25"));
    }
    #[test]
    fn parses_protonvpn_1_conf() {
        // protonvpn-1.conf
        let proton_config = r#"[Interface]
# Key for TII
# Bouncing = 4
# NAT-PMP (Port Forwarding) = off
# VPN Accelerator = on
Address = 10.2.0.2/32
DNS = 10.2.0.1

[Peer]
# NL-FREE#241
PublicKey = veNAbXSWZO0nj6duvLa6C0yzEkR/MP994IvjzoJfDxs=
AllowedIPs = 0.0.0.0/0, ::/0
Endpoint = 185.183.34.41:51820"#;

        let cfg = parse_config(proton_config).expect("Should parse protonvpn-1.conf");

        // Interface
        assert_eq!(cfg.interface.name.as_deref(), Some("TII"));
        assert_eq!(cfg.interface.address.as_deref(), Some("10.2.0.2/32"));
        assert_eq!(cfg.interface.dns.as_deref(), Some("10.2.0.1"));

        // Peer
        assert_eq!(cfg.peers.len(), 1);
        let peer = &cfg.peers[0];
        assert_eq!(peer.name.as_deref(), Some("NL-FREE#241"));
        assert_eq!(
            peer.public_key.as_deref(),
            Some("veNAbXSWZO0nj6duvLa6C0yzEkR/MP994IvjzoJfDxs=")
        );
        assert_eq!(peer.allowed_ips.as_deref(), Some("0.0.0.0/0, ::/0"));
        assert_eq!(peer.endpoint.as_deref(), Some("185.183.34.41:51820"));
    }

    #[test]
    fn parses_protonvpn_2_conf() {
        // protonvpn-2.conf (German localization)
        let proton_config = r#"[Interface]
# Key for ProtonG
# Bouncing = 20
# NetShield = 1
# Moderates NAT = off
# NAT-PMP (Port-Weiterleitung) = off
# VPN-Accelerator = on
Address = 10.2.0.2/32
DNS = 10.2.0.1
[Peer]
# AT#133
PublicKey = D2G0wjy9kRvxjXJfLCA9zglgcwMvZFMhLG+NASSzQ1k=
AllowedIPs = 0.0.0.0/0, ::/0"#;

        let cfg = parse_config(proton_config).expect("Should parse protonvpn-2.conf");

        // Interface
        assert_eq!(cfg.interface.name.as_deref(), Some("ProtonG"));
        assert_eq!(cfg.interface.address.as_deref(), Some("10.2.0.2/32"));
        assert_eq!(cfg.interface.dns.as_deref(), Some("10.2.0.1"));

        // Peer
        assert_eq!(cfg.peers.len(), 1);
        let peer = &cfg.peers[0];
        assert_eq!(peer.name.as_deref(), Some("AT#133"));
        assert_eq!(
            peer.public_key.as_deref(),
            Some("D2G0wjy9kRvxjXJfLCA9zglgcwMvZFMhLG+NASSzQ1k=")
        );
        assert_eq!(peer.allowed_ips.as_deref(), Some("0.0.0.0/0, ::/0"));
        assert_eq!(peer.endpoint, None);
    }
}
