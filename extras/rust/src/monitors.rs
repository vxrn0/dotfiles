use serde_json::Value;
use std::process::Command;

pub struct Monitors {
    pub names: Vec<String>,
    sizes: Vec<(u64, u64)>,
}

impl Monitors {
    pub fn min_size(&self) -> Result<(u64, u64), String> {
        let width = self
            .sizes
            .iter()
            .map(|(width, _)| *width)
            .min()
            .ok_or("hyprctl returned no monitors")?;
        let height = self
            .sizes
            .iter()
            .map(|(_, height)| *height)
            .min()
            .ok_or("hyprctl returned no monitors")?;
        Ok((width, height))
    }
}

pub fn load() -> Result<Monitors, String> {
    let output = Command::new("hyprctl")
        .args(["monitors", "-j"])
        .output()
        .map_err(|error| format!("cannot run hyprctl monitors -j: {error}"))?;
    if !output.status.success() {
        return Err(format!("hyprctl monitors -j failed: {}", output.status));
    }
    let text = String::from_utf8(output.stdout)
        .map_err(|_| "hyprctl monitors -j returned non-UTF-8 output".to_string())?;
    parse(&text)
}

pub fn parse(text: &str) -> Result<Monitors, String> {
    let value: Value = serde_json::from_str(text)
        .map_err(|error| format!("hyprctl monitors -j returned invalid JSON: {error}"))?;
    let entries = value
        .as_array()
        .ok_or("hyprctl monitors -j did not return an array")?;
    let mut names = Vec::with_capacity(entries.len());
    let mut sizes = Vec::with_capacity(entries.len());
    for entry in entries {
        let name = entry
            .get("name")
            .and_then(Value::as_str)
            .ok_or("a monitor has no name")?;
        let width = entry
            .get("width")
            .and_then(Value::as_u64)
            .ok_or_else(|| format!("monitor {name} has no integer width"))?;
        let height = entry
            .get("height")
            .and_then(Value::as_u64)
            .ok_or_else(|| format!("monitor {name} has no integer height"))?;
        names.push(name.to_string());
        sizes.push((width, height));
    }
    if names.is_empty() {
        return Err("hyprctl returned no monitors".to_string());
    }
    Ok(Monitors { names, sizes })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_names_and_smallest_logical_size() {
        let monitors = parse(
            r#"[
                {"name":"HDMI-A-1","width":1920,"height":1080},
                {"name":"DP-1","width":2560,"height":1440}
            ]"#,
        )
        .unwrap();
        assert_eq!(monitors.names, vec!["HDMI-A-1", "DP-1"]);
        assert_eq!(monitors.min_size().unwrap(), (1920, 1080));
    }

    #[test]
    fn rejects_bad_monitor_json() {
        assert!(parse("{}").is_err());
        assert!(parse("[]").is_err());
        assert!(parse(r#"[{"name":"x","width":"wide","height":1}]"#).is_err());
    }
}
