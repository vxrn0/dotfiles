use serde_json::{Map, Value};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

pub struct ShellConfig {
    path: PathBuf,
    doc: Option<Value>,
}

impl ShellConfig {
    pub fn load(path: &Path) -> Result<Self, String> {
        let doc = match fs::read_to_string(path) {
            Ok(text) => Some(
                serde_json::from_str(&text)
                    .map_err(|error| format!("{}: invalid JSON: {error}", path.display()))?,
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(format!("cannot read {}: {error}", path.display())),
        };
        Ok(Self {
            path: path.to_path_buf(),
            doc,
        })
    }

    /// jq -r equivalent for strings, numbers and booleans. Missing and null
    /// values have no scalar form.
    pub fn scalar(&self, keys: &[&str]) -> Option<String> {
        let mut value = self.doc.as_ref()?;
        for key in keys {
            value = value.get(*key)?;
        }
        match value {
            Value::String(text) => Some(text.clone()),
            Value::Number(number) => Some(number.to_string()),
            Value::Bool(flag) => Some(flag.to_string()),
            _ => None,
        }
    }

    /// Atomic replacement like mktemp+jq+mv in the script. A missing config
    /// file makes the write a no-op, as in the script.
    pub fn set_string(&mut self, keys: &[&str], value: &str) -> Result<(), String> {
        let Some(doc) = self.doc.as_mut() else {
            return Ok(());
        };
        set_nested(doc, keys, Value::String(value.to_string()));
        let text = serde_json::to_string_pretty(doc)
            .map_err(|error| format!("cannot serialize {}: {error}", self.path.display()))?;
        atomic_write(&self.path, format!("{text}\n").as_bytes(), 0o600)
    }
}

fn set_nested(current: &mut Value, keys: &[&str], value: Value) {
    if keys.is_empty() {
        *current = value;
        return;
    }
    if !current.is_object() {
        *current = Value::Object(Map::new());
    }
    let object = current.as_object_mut().expect("object just installed");
    if keys.len() == 1 {
        object.insert(keys[0].to_string(), value);
    } else {
        let child = object
            .entry(keys[0])
            .or_insert_with(|| Value::Object(Map::new()));
        set_nested(child, &keys[1..], value);
    }
}

pub fn atomic_write(path: &Path, data: &[u8], mode: u32) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("{} has no parent directory", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    let name = path
        .file_name()
        .ok_or_else(|| format!("{} has no file name", path.display()))?
        .to_string_lossy();
    for attempt in 0..1000 {
        let temp = parent.join(format!(".{name}.{}.{attempt}", std::process::id()));
        let mut file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(mode)
            .open(&temp)
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!("cannot create {}: {error}", temp.display()));
            }
        };
        if let Err(error) = file.write_all(data) {
            let _ = fs::remove_file(&temp);
            return Err(format!("cannot write {}: {error}", temp.display()));
        }
        drop(file);
        if let Err(error) = fs::rename(&temp, path) {
            let _ = fs::remove_file(&temp);
            return Err(format!("cannot replace {}: {error}", path.display()));
        }
        return Ok(());
    }
    Err(format!(
        "cannot create a temporary file next to {}",
        path.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "dots-theme-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn writes_the_same_keys_and_order_as_jq() {
        let dir = temp_dir("config");
        let path = dir.join("config.json");
        fs::write(
            &path,
            "{\n  \"b\": 1,\n  \"wallpaper\": {\n    \"path\": \"old\",\n    \"thumbnailPath\": \"\"\n  },\n  \"a\": \"x\"\n}\n",
        )
        .unwrap();
        let mut config = ShellConfig::load(&path).unwrap();
        config
            .set_string(&["wallpaper", "path"], "/new.png")
            .unwrap();
        config
            .set_string(&["appearance", "palette", "accentColor"], "")
            .unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert_eq!(
            text,
            "{\n  \"b\": 1,\n  \"wallpaper\": {\n    \"path\": \"/new.png\",\n    \"thumbnailPath\": \"\"\n  },\n  \"a\": \"x\",\n  \"appearance\": {\n    \"palette\": {\n      \"accentColor\": \"\"\n    }\n  }\n}\n"
        );
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn missing_config_is_a_noop_and_missing_keys_return_none() {
        let dir = temp_dir("missing");
        let path = dir.join("config.json");
        let mut config = ShellConfig::load(&path).unwrap();
        config.set_string(&["wallpaper", "path"], "/x").unwrap();
        assert!(!path.exists());
        assert_eq!(config.scalar(&["wallpaper", "path"]), None);

        fs::write(
            &path,
            "{\"appearance\":{\"palette\":{\"type\":\"scheme-neutral\"}}}",
        )
        .unwrap();
        let config = ShellConfig::load(&path).unwrap();
        assert_eq!(
            config.scalar(&["appearance", "palette", "type"]).as_deref(),
            Some("scheme-neutral")
        );
        assert_eq!(
            config.scalar(&["appearance", "palette", "accentColor"]),
            None
        );
        let _ = fs::remove_dir_all(dir);
    }
}
