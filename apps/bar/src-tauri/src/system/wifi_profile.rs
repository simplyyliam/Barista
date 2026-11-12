//! The Wi-Fi profile (XML) Windows needs to join a new network from the bar. Pure text building,
//! so it is tested without touching the radio. The password goes into the XML and nowhere else:
//! it is never logged or kept (CLAUDE.md rule 3).

/// How a network protects itself, as far as we can join it from the bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Auth {
    Open,
    /// WPA (TKIP/AES), the old personal mode.
    WpaPsk,
    Wpa2Psk,
    Wpa3Sae,
}

impl Auth {
    /// From Windows' `DOT11_AUTH_ALGORITHM` number. Enterprise networks (802.1X) return `None`:
    /// they need a certificate or account, so the bar opens Windows' own flyout for those.
    pub fn from_algo(algo: i32) -> Option<Self> {
        match algo {
            1 => Some(Self::Open),
            4 => Some(Self::WpaPsk),
            7 => Some(Self::Wpa2Psk),
            9 => Some(Self::Wpa3Sae),
            _ => None,
        }
    }

    pub fn needs_password(self) -> bool {
        self != Self::Open
    }
}

/// Longest SSID the standard allows, in bytes.
pub const MAX_SSID_BYTES: usize = 32;

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c => out.push(c),
        }
    }
    out
}

/// A WPA passphrase is 8 to 63 characters (a 64 digit hex key is also allowed). Nothing with
/// control characters.
pub fn password_ok(password: &str) -> bool {
    let n = password.chars().count();
    (8..=63).contains(&n) && !password.chars().any(char::is_control)
}

/// The profile for `ssid`. `None` if the name or password can't be right.
pub fn profile_xml(ssid: &str, auth: Auth, password: Option<&str>) -> Option<String> {
    if ssid.is_empty() || ssid.len() > MAX_SSID_BYTES || ssid.chars().any(char::is_control) {
        return None;
    }
    let hex: String = ssid.bytes().map(|b| format!("{b:02X}")).collect();
    let name = escape(ssid);
    let (authentication, encryption) = match auth {
        Auth::Open => ("open", "none"),
        Auth::WpaPsk => ("WPAPSK", "AES"),
        Auth::Wpa2Psk => ("WPA2PSK", "AES"),
        Auth::Wpa3Sae => ("WPA3SAE", "AES"),
    };
    let key = if auth.needs_password() {
        let password = password.filter(|p| password_ok(p))?;
        format!(
            "<sharedKey><keyType>passPhrase</keyType><protected>false</protected><keyMaterial>{}</keyMaterial></sharedKey>",
            escape(password)
        )
    } else {
        String::new()
    };
    Some(format!(
        "<?xml version=\"1.0\"?><WLANProfile xmlns=\"http://www.microsoft.com/networking/WLAN/profile/v1\">\
<name>{name}</name><SSIDConfig><SSID><hex>{hex}</hex><name>{name}</name></SSID></SSIDConfig>\
<connectionType>ESS</connectionType><connectionMode>auto</connectionMode>\
<MSM><security><authEncryption><authentication>{authentication}</authentication>\
<encryption>{encryption}</encryption><useOneX>false</useOneX></authEncryption>{key}</security></MSM></WLANProfile>"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wpa2_profile_has_the_key_and_hex_name() {
        let xml = profile_xml("Home", Auth::Wpa2Psk, Some("correct horse")).unwrap();
        assert!(xml.contains("<hex>486F6D65</hex>"));
        assert!(xml.contains("<authentication>WPA2PSK</authentication>"));
        assert!(xml.contains("<keyMaterial>correct horse</keyMaterial>"));
    }

    #[test]
    fn open_profile_has_no_key() {
        let xml = profile_xml("Cafe", Auth::Open, None).unwrap();
        assert!(xml.contains("<authentication>open</authentication>"));
        assert!(!xml.contains("sharedKey"));
    }

    #[test]
    fn xml_characters_are_escaped() {
        let xml = profile_xml("a<b>&\"c", Auth::Wpa2Psk, Some("p&ss<word>1")).unwrap();
        assert!(xml.contains("<name>a&lt;b&gt;&amp;&quot;c</name>"));
        assert!(xml.contains("p&amp;ss&lt;word&gt;1"));
        assert!(!xml.contains("a<b>"));
    }

    #[test]
    fn bad_input_is_refused() {
        assert!(profile_xml("", Auth::Open, None).is_none());
        assert!(profile_xml(&"x".repeat(33), Auth::Open, None).is_none());
        assert!(profile_xml("net\n", Auth::Open, None).is_none());
        assert!(profile_xml("Home", Auth::Wpa2Psk, None).is_none());
        assert!(profile_xml("Home", Auth::Wpa2Psk, Some("short")).is_none());
        assert!(profile_xml("Home", Auth::Wpa2Psk, Some(&"x".repeat(64))).is_none());
        assert!(profile_xml("Home", Auth::Wpa2Psk, Some("bad\u{0}password")).is_none());
    }

    #[test]
    fn enterprise_networks_are_not_joinable() {
        assert_eq!(Auth::from_algo(6), None);
        assert_eq!(Auth::from_algo(7), Some(Auth::Wpa2Psk));
        assert_eq!(Auth::from_algo(9), Some(Auth::Wpa3Sae));
    }
}
