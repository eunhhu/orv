//! Shared JSON object-shape validation for build artifacts and editor protocols.

pub fn verify_json_object_keys_exact(
    value: &serde_json::Value,
    expected: &[&str],
    context: &str,
) -> anyhow::Result<()> {
    verify_json_object_keys_allowing_optional(value, expected, &[], context)
}

pub fn verify_json_object_keys_allowing_optional(
    value: &serde_json::Value,
    required: &[&str],
    optional: &[&str],
    context: &str,
) -> anyhow::Result<()> {
    let object = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("{context} must be an object"))?;
    if required.iter().any(|key| !object.contains_key(*key))
        || object
            .keys()
            .any(|key| !required.contains(&key.as_str()) && !optional.contains(&key.as_str()))
    {
        anyhow::bail!("{context} keys must match contract");
    }
    Ok(())
}
