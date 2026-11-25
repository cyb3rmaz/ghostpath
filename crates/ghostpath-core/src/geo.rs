use serde::{Deserialize, Serialize};

/// Mode used to determine the target country for system Geo Location and Timezone spoofing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum GeoSpoofMode {
    /// Automatically match the country of the active connected VPN server node.
    #[default]
    MatchNode,
    /// Use a manually chosen country preset from the country list.
    Manual,
}

/// Static geographic coordinates written for desktop GeoClue2 static-source spoofing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeoCoordinates {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_m: f64,
    pub accuracy_m: f64,
}

/// Curated country entry mapping ISO-3166 country codes to verified IANA timezones and coordinates.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeoCountryPreset {
    pub country_code: String,
    pub country_name: String,
    pub city: String,
    pub timezone: String,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_m: f64,
    pub accuracy_m: f64,
    pub utc_offset_hint: String,
}

impl GeoCountryPreset {
    pub fn coordinates(&self) -> GeoCoordinates {
        GeoCoordinates {
            latitude: self.latitude,
            longitude: self.longitude,
            altitude_m: self.altitude_m,
            accuracy_m: self.accuracy_m,
        }
    }
}

/// Persisted configuration for Geo Location & System Timezone spoofing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeoSpoofSettings {
    pub enabled: bool,
    pub mode: GeoSpoofMode,
    pub country_code: String,
    pub spoof_timezone: bool,
    pub spoof_geolocation: bool,
    pub apply_on_connect: bool,
    pub strict_geo_routing: bool,
}

impl Default for GeoSpoofSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: GeoSpoofMode::MatchNode,
            country_code: "DE".to_string(),
            spoof_timezone: true,
            spoof_geolocation: true,
            apply_on_connect: true,
            strict_geo_routing: false,
        }
    }
}

/// Live runtime status snapshot reported by `ghostpathd` to GUI and CLI clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeoSpoofStatus {
    pub settings: GeoSpoofSettings,
    pub spoof_active: bool,
    pub applied_country_code: Option<String>,
    pub applied_country_name: Option<String>,
    pub applied_timezone: Option<String>,
    pub active_timezone: String,
    pub active_geolocation: Option<GeoCoordinates>,
    pub backed_up_timezone: Option<String>,
    pub geoclue_available: bool,
    pub timedatectl_available: bool,
}

fn preset(
    country_code: &str,
    country_name: &str,
    city: &str,
    timezone: &str,
    latitude: f64,
    longitude: f64,
    utc_offset_hint: &str,
) -> GeoCountryPreset {
    GeoCountryPreset {
        country_code: country_code.to_string(),
        country_name: country_name.to_string(),
        city: city.to_string(),
        timezone: timezone.to_string(),
        latitude,
        longitude,
        altitude_m: 35.0,
        accuracy_m: 150.0,
        utc_offset_hint: utc_offset_hint.to_string(),
    }
}

/// Returns the built-in catalog of supported countries with verified IANA timezones and city coordinates.
pub fn geo_country_presets() -> Vec<GeoCountryPreset> {
    vec![
        preset("DE", "Germany", "Frankfurt", "Europe/Berlin", 50.1109, 8.6821, "UTC+01:00"),
        preset("NL", "Netherlands", "Amsterdam", "Europe/Amsterdam", 52.3676, 4.9041, "UTC+01:00"),
        preset("US", "United States", "New York", "America/New_York", 40.7128, -74.0060, "UTC-05:00"),
        preset("GB", "United Kingdom", "London", "Europe/London", 51.5074, -0.1278, "UTC+00:00"),
        preset("FI", "Finland", "Helsinki", "Europe/Helsinki", 60.1699, 24.9384, "UTC+02:00"),
        preset("FR", "France", "Paris", "Europe/Paris", 48.8566, 2.3522, "UTC+01:00"),
        preset("SE", "Sweden", "Stockholm", "Europe/Stockholm", 59.3293, 18.0686, "UTC+01:00"),
        preset("CH", "Switzerland", "Zurich", "Europe/Zurich", 47.3769, 8.5417, "UTC+01:00"),
        preset("AT", "Austria", "Vienna", "Europe/Vienna", 48.2082, 16.3738, "UTC+01:00"),
        preset("PL", "Poland", "Warsaw", "Europe/Warsaw", 52.2297, 21.0122, "UTC+01:00"),
        preset("ES", "Spain", "Madrid", "Europe/Madrid", 40.4168, -3.7038, "UTC+01:00"),
        preset("IT", "Italy", "Milan", "Europe/Rome", 45.4642, 9.1900, "UTC+01:00"),
        preset("NO", "Norway", "Oslo", "Europe/Oslo", 59.9139, 10.7522, "UTC+01:00"),
        preset("IE", "Ireland", "Dublin", "Europe/Dublin", 53.3498, -6.2603, "UTC+00:00"),
        preset("RO", "Romania", "Bucharest", "Europe/Bucharest", 44.4268, 26.1025, "UTC+02:00"),
        preset("EE", "Estonia", "Tallinn", "Europe/Tallinn", 59.4370, 24.7536, "UTC+02:00"),
        preset("TR", "Turkey", "Istanbul", "Europe/Istanbul", 41.0082, 28.9784, "UTC+03:00"),
        preset("AE", "United Arab Emirates", "Dubai", "Asia/Dubai", 25.2048, 55.2708, "UTC+04:00"),
        preset("CA", "Canada", "Toronto", "America/Toronto", 43.6532, -79.3832, "UTC-05:00"),
        preset("BR", "Brazil", "Sao Paulo", "America/Sao_Paulo", -23.5505, -46.6333, "UTC-03:00"),
        preset("SG", "Singapore", "Singapore", "Asia/Singapore", 1.3521, 103.8198, "UTC+08:00"),
        preset("JP", "Japan", "Tokyo", "Asia/Tokyo", 35.6762, 139.6503, "UTC+09:00"),
        preset("KR", "South Korea", "Seoul", "Asia/Seoul", 37.5665, 126.9780, "UTC+09:00"),
        preset("HK", "Hong Kong", "Hong Kong", "Asia/Hong_Kong", 22.3193, 114.1694, "UTC+08:00"),
        preset("IN", "India", "Mumbai", "Asia/Kolkata", 19.0760, 72.8777, "UTC+05:30"),
        preset("AU", "Australia", "Sydney", "Australia/Sydney", -33.8688, 151.2093, "UTC+10:00"),
        preset("RU", "Russia", "Moscow", "Europe/Moscow", 55.7558, 37.6173, "UTC+03:00"),
        preset("UA", "Ukraine", "Kyiv", "Europe/Kyiv", 50.4501, 30.5234, "UTC+02:00"),
        preset("IR", "Iran", "Tehran", "Asia/Tehran", 35.6892, 51.3890, "UTC+03:30"),
    ]
}

/// Finds a country preset by ISO alpha-2 code, common alias, country name, or city name.
pub fn find_country_preset(query: &str) -> Option<GeoCountryPreset> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return None;
    }

    let normalized = match trimmed.to_ascii_uppercase().as_str() {
        "UK" => "GB".to_string(),
        "UAE" => "AE".to_string(),
        "USA" => "US".to_string(),
        "HOLLAND" => "NL".to_string(),
        other => other.to_string(),
    };

    let lower = trimmed.to_ascii_lowercase();
    let presets = geo_country_presets();

    presets
        .iter()
        .find(|p| p.country_code.eq_ignore_ascii_case(&normalized))
        .or_else(|| {
            presets.iter().find(|p| {
                p.country_name.to_ascii_lowercase() == lower
                    || p.city.to_ascii_lowercase() == lower
            })
        })
        .cloned()
}

/// Validates that an IANA timezone string contains only safe path segments (no traversal).
pub fn validate_timezone_name(tz: &str) -> bool {
    if tz.is_empty() || tz.len() > 64 || tz.starts_with('/') || tz.ends_with('/') {
        return false;
    }
    if tz.contains("..") {
        return false;
    }
    tz.bytes().all(|b| {
        b.is_ascii_alphanumeric() || matches!(b, b'/' | b'_' | b'-' | b'+')
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_lookup_and_aliases() {
        let de = find_country_preset("de").unwrap();
        assert_eq!(de.timezone, "Europe/Berlin");
        assert_eq!(de.country_name, "Germany");

        let uk = find_country_preset("UK").unwrap();
        assert_eq!(uk.country_code, "GB");
        assert_eq!(uk.timezone, "Europe/London");

        let tokyo = find_country_preset("Tokyo").unwrap();
        assert_eq!(tokyo.country_code, "JP");

        assert!(find_country_preset("UN").is_none());
        assert!(find_country_preset("").is_none());
    }

    #[test]
    fn test_timezone_validation_rejects_traversal() {
        assert!(validate_timezone_name("Europe/Berlin"));
        assert!(validate_timezone_name("America/Sao_Paulo"));
        assert!(validate_timezone_name("Etc/GMT+3"));
        assert!(!validate_timezone_name(""));
        assert!(!validate_timezone_name("../etc/passwd"));
        assert!(!validate_timezone_name("/Europe/Berlin"));
        assert!(!validate_timezone_name("Europe/Berlin;rm -rf"));
    }

    #[test]
    fn test_all_presets_have_valid_timezones_and_coords() {
        let presets = geo_country_presets();
        assert!(presets.len() >= 25);
        for p in presets {
            assert!(validate_timezone_name(&p.timezone), "Invalid tz: {}", p.timezone);
            assert!((-90.0..=90.0).contains(&p.latitude));
            assert!((-180.0..=180.0).contains(&p.longitude));
        }
    }
}
