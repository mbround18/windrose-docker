// src/json_config.rs
// Handles append-only JSON configuration merging.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use serde_json::{Map, Value};

/// Helper function for deep merging two JSON values.
/// Scalar values in `source` overwrite `target`.
/// Objects are merged recursively. Arrays are overwritten.
fn deep_merge(target: &mut Value, source: &Value) {
    match (target, source) {
        (Value::Object(target_map), Value::Object(source_map)) => {
            for (key, source_value) in source_map {
                if let Some(target_value) = target_map.get_mut(key) {
                    deep_merge(target_value, source_value);
                } else {
                    target_map.insert(key.clone(), source_value.clone());
                }
            }
        }
        (target, source) => {
            // Overwrite if not both objects (e.g., merging an object with a scalar, or two arrays)
            *target = source.clone();
        }
    }
}

/// Applies environment variables to a JSON file using an append-only deep-merge strategy.
///
/// This function performs the following steps:
/// 1. Loads an existing JSON file or creates an empty JSON object if the file does not exist.
/// 2. Creates a backup of the original JSON file before any modifications are made.
/// 3. Iterates through the provided `mapping_table`, which maps environment variable names
///    to their corresponding JSON key paths (e.g., "WINDROSE_SERVER_NAME" to "ServerDescription_Persistent.ServerName").
/// 4. For each matching environment variable, it constructs a nested JSON fragment and deep-merges
///    it into the `existing_json`. This ensures that existing unrelated keys are preserved
///    and only the specified fields are updated.
/// 5. Writes the modified JSON to a temporary file, then atomically renames it to the
///    original file path to prevent data corruption.
pub fn apply_env_to_json(
    env_vars: &HashMap<String, String>,
    json_file_path: &Path,
    mapping_table: &HashMap<String, String>, // Maps env_var_name to json_path (e.g., "ServerDescription_Persistent.ServerName")
) -> Result<()> {
    // 1. Load existing JSON or create if missing
    let mut existing_json: Value = if json_file_path.exists() {
        let content = fs::read_to_string(json_file_path)?;
        serde_json::from_str(&content).unwrap_or_else(|_| Value::Object(Map::new())) // Handle malformed JSON gracefully
    } else {
        Value::Object(Map::new())
    };

    // 2. Create a backup of the original JSON before modification
    let backup_path = PathBuf::from(json_file_path).with_extension("json.bak");
    if json_file_path.exists() {
        fs::copy(json_file_path, &backup_path)?;
    }

    // 3. For each WINDROSE_* env var, map to JSON key path and deep-merge
    for (env_var_key, json_key_path) in mapping_table {
        if let Some(env_value) = env_vars.get(env_var_key) {
            let mut new_json_fragment = Value::Object(Map::new());
            let mut current_fragment_ptr = &mut new_json_fragment;

            let parts: Vec<&str> = json_key_path.split('.').collect();
            for (i, part) in parts.iter().enumerate() {
                if i < parts.len() - 1 {
                    current_fragment_ptr = current_fragment_ptr
                        .as_object_mut()
                        .ok_or_else(|| anyhow::anyhow!("Invalid JSON structure for path: {}", json_key_path))?
                        .entry(*part)
                        .or_insert_with(|| Value::Object(Map::new()));
                } else {
                    // Convert env_value to appropriate JSON type (e.g., bool, number) or default to string
                    let final_value = if let Ok(b) = env_value.parse::<bool>() {
                        Value::Bool(b)
                    } else if let Ok(n) = env_value.parse::<i64>() {
                        Value::Number(n.into())
                    } else if let Ok(f) = env_value.parse::<f64>() {
                        Value::Number(serde_json::Number::from_f64(f).unwrap())
                    } else {
                        Value::String(env_value.clone())
                    };
                    current_fragment_ptr
                        .as_object_mut()
                        .ok_or_else(|| anyhow::anyhow!("Invalid JSON structure for path: {}", json_key_path))?
                        .insert(part.to_string(), final_value);
                }
            }
            deep_merge(&mut existing_json, &new_json_fragment);
        }
    }

    // 4. Write to a temporary file, then atomically rename to prevent data corruption
    let temp_path = PathBuf::from(json_file_path).with_extension("json.tmp");
    fs::write(&temp_path, serde_json::to_string_pretty(&existing_json)?)?;
    fs::rename(&temp_path, json_file_path)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::io::Write;

    #[test]
    fn test_deep_merge_overwrite_scalar() {
        let mut target: Value = serde_json::from_str(r#"{"a": 1, "b": {"c": 2}}"#).unwrap();
        let source: Value = serde_json::from_str(r#"{"a": 2}"#).unwrap();
        deep_merge(&mut target, &source);
        assert_eq!(target, serde_json::from_str::<Value>(r#"{"a": 2, "b": {"c": 2}}"#).unwrap());
    }

    #[test]
    fn test_deep_merge_add_new_key() {
        let mut target: Value = serde_json::from_str(r#"{"a": 1}"#).unwrap();
        let source: Value = serde_json::from_str(r#"{"b": 2}"#).unwrap();
        deep_merge(&mut target, &source);
        assert_eq!(target, serde_json::from_str::<Value>(r#"{"a": 1, "b": 2}"#).unwrap());
    }

    #[test]
    fn test_deep_merge_recursive() {
        let mut target: Value = serde_json::from_str(r#"{"a": {"b": 1, "c": 2}}"#).unwrap();
        let source: Value = serde_json::from_str(r#"{"a": {"b": 3, "d": 4}}"#).unwrap();
        deep_merge(&mut target, &source);
        assert_eq!(target, serde_json::from_str::<Value>(r#"{"a": {"b": 3, "c": 2, "d": 4}}"#).unwrap());
    }

    #[test]
    fn test_deep_merge_idempotency() {
        let mut target: Value = serde_json::from_str(r#"{"a": 1}"#).unwrap();
        let source: Value = serde_json::from_str(r#"{"a": 2}"#).unwrap();
        deep_merge(&mut target, &source);
        assert_eq!(target, serde_json::from_str::<Value>(r#"{"a": 2}"#).unwrap());
        deep_merge(&mut target, &source); // Apply again
        assert_eq!(target, serde_json::from_str::<Value>(r#"{"a": 2}"#).unwrap());
    }

    #[test]
    fn test_apply_env_to_json_new_file() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("config.json");
        let mut env_vars = HashMap::new();
        env_vars.insert("WINDROSE_SERVER_NAME".to_string(), "MyServer".to_string());
        env_vars.insert("WINDROSE_MAX_PLAYERS".to_string(), "10".to_string());

        let mut mapping = HashMap::new();
        mapping.insert("WINDROSE_SERVER_NAME".to_string(), "ServerDescription_Persistent.ServerName".to_string());
        mapping.insert("WINDROSE_MAX_PLAYERS".to_string(), "ServerDescription_Persistent.MaxPlayerCount".to_string());

        apply_env_to_json(&env_vars, &file_path, &mapping)?;

        let content = fs::read_to_string(&file_path)?;
        let expected: Value = serde_json::from_str::<Value>(
            r#"{
                "ServerDescription_Persistent": {
                    "ServerName": "MyServer",
                    "MaxPlayerCount": 10
                }
            }"#,
        )?;
        assert_eq!(serde_json::from_str::<Value>(&content)?, expected);
        assert!(!dir.path().join("config.json.bak").exists()); // No backup if file didn't exist

        Ok(())
    }

    #[test]
    fn test_apply_env_to_json_existing_file() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("config.json");
        let initial_content = r#"{
            "ServerDescription_Persistent": {
                "ServerName": "OldServer",
                "UserSelectedRegion": "US"
            },
            "OtherSetting": "Value"
        }"#;
        let mut file = fs::File::create(&file_path)?;
        file.write_all(initial_content.as_bytes())?;

        let mut env_vars = HashMap::new();
        env_vars.insert("WINDROSE_SERVER_NAME".to_string(), "NewServer".to_string());
        env_vars.insert("WINDROSE_MAX_PLAYERS".to_string(), "20".to_string());

        let mut mapping = HashMap::new();
        mapping.insert("WINDROSE_SERVER_NAME".to_string(), "ServerDescription_Persistent.ServerName".to_string());
        mapping.insert("WINDROSE_MAX_PLAYERS".to_string(), "ServerDescription_Persistent.MaxPlayerCount".to_string());

        apply_env_to_json(&env_vars, &file_path, &mapping)?;

        let content = fs::read_to_string(&file_path)?;
        let expected: Value = serde_json::from_str::<Value>(
            r#"{
                "ServerDescription_Persistent": {
                    "ServerName": "NewServer",
                    "UserSelectedRegion": "US",
                    "MaxPlayerCount": 20
                },
                "OtherSetting": "Value"
            }"#,
        )?;
        assert_eq!(serde_json::from_str::<Value>(&content)?, expected);

        // Verify backup
        let backup_content = fs::read_to_string(dir.path().join("config.json.bak"))?;
        assert_eq!(serde_json::from_str::<Value>(&backup_content)?, serde_json::from_str::<Value>(initial_content)?);

        Ok(())
    }

    #[test]
    fn test_apply_env_to_json_type_conversion() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("config.json");
        
        let mut env_vars = HashMap::new();
        env_vars.insert("WINDROSE_DIRECT_CONNECTION".to_string(), "true".to_string());
        env_vars.insert("WINDROSE_DIRECT_PORT".to_string(), "7777".to_string());

        let mut mapping = HashMap::new();
        mapping.insert("WINDROSE_DIRECT_CONNECTION".to_string(), "ServerDescription_Persistent.UseDirectConnection".to_string());
        mapping.insert("WINDROSE_DIRECT_PORT".to_string(), "ServerDescription_Persistent.DirectConnectionServerPort".to_string());

        apply_env_to_json(&env_vars, &file_path, &mapping)?;

        let content = fs::read_to_string(&file_path)?;
        let expected: Value = serde_json::from_str::<Value>(
            r#"{
                "ServerDescription_Persistent": {
                    "UseDirectConnection": true,
                    "DirectConnectionServerPort": 7777
                }
            }"#,
        )?;
        assert_eq!(serde_json::from_str::<Value>(&content)?, expected);

        Ok(())
    }

    #[test]
    fn test_apply_env_to_json_malformed_initial_json() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("malformed.json");
        fs::write(&file_path, "this is not valid json")?;

        let mut env_vars = HashMap::new();
        env_vars.insert("WINDROSE_SERVER_NAME".to_string(), "TestServer".to_string());
        let mut mapping = HashMap::new();
        mapping.insert("WINDROSE_SERVER_NAME".to_string(), "ServerName".to_string());

        apply_env_to_json(&env_vars, &file_path, &mapping)?;

        let content = fs::read_to_string(&file_path)?;
        let expected: Value = serde_json::from_str::<Value>(r#"{"ServerName": "TestServer"}"#)?;
        assert_eq!(serde_json::from_str::<Value>(&content)?, expected);

        // Backup should contain the malformed content
        let backup_content = fs::read_to_string(dir.path().join("malformed.json.bak"))?;
        assert_eq!(backup_content, "this is not valid json");

        Ok(())
    }
}
