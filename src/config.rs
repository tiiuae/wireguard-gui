use serde::{Serialize, Deserialize};

/// Defines the VPN settings for the local node.
#[derive(Deserialize, Serialize, Clone, PartialEq, Default, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Interface {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub listen_port: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_key: Option<String>,
    #[serde(rename = "DNS", skip_serializing_if = "Option::is_none")]
    pub dns: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table: Option<String>,
    #[serde(rename = "MTU", skip_serializing_if = "Option::is_none")]
    pub mtu: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_up: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_up: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_down: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_down: Option<String>,
}

/// Defines the VPN settings for a remote peer capable of routing
/// traffic for one or more addresses (itself and/or other
/// peers). Peers can be either a public bounce server that relays
/// traffic to other peers, or a directly accessible client via
/// LAN/internet that is not behind a NAT and only routes traffic for
/// itself.
#[derive(Deserialize, Serialize, Clone, PartialEq, Default, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Peer {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "AllowedIPs", skip_serializing_if = "Option::is_none")]
    pub allowed_ips: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persistent_keepalive: Option<String>,

}

#[derive(Deserialize, Serialize, Clone, PartialEq, Default, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct WireguardConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    interface: Option<Interface>,
    #[serde(skip_serializing_if = "Option::is_none")]
    peer: Option<Peer>,
}

pub fn parse_config(s: &str) -> Result<WireguardConfig, serde_ini::de::Error> {
    serde_ini::from_str::<WireguardConfig>(s)
}

pub fn write_config(c: &WireguardConfig) -> Result<String, serde_ini::ser::Error> {
    serde_ini::to_string(c).map(|s| s.replace("\r\n", "\n"))
}

mod tests {
    use super::{parse_config, write_config};

    #[test]
    fn deserialization_serialization() {
        const TEST_INPUT: &str = "[Interface]
Address=10.0.0.1/24
ListenPort=51820
PrivateKey=<contents-of-server-privatekey>
PostUp=iptables -A FORWARD -i wg0 -j ACCEPT; iptables -t nat -A POSTROUTING -o eth0 -j MASQUERADE
PostDown=iptables -D FORWARD -i wg0 -j ACCEPT; iptables -t nat -D POSTROUTING -o eth0 -j MASQUERADE
[Peer]
AllowedIPs=10.0.0.2/32
PublicKey=<contents-of-client-publickey>
";

        let parsed = parse_config(TEST_INPUT).unwrap();
        let written = write_config(&parsed).unwrap();
        let parsed_again = parse_config(&written).unwrap();

        assert_eq!(TEST_INPUT, &written);
        assert_eq!(parsed, parsed_again);
    }
}
