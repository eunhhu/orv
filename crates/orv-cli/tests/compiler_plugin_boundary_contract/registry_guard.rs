use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::{validate_metadata, validate_owner_package, validate_surface};

pub(super) fn validate_plugin_registry(inventory: &Value, errors: &mut Vec<String>) {
    let Some(registry) = inventory["plugin_registry"].as_array() else {
        errors.push("plugin_registry missing".to_string());
        return;
    };
    let expected = orv_hir::domain_plugin_registry();
    if registry.len() != expected.len() {
        errors.push(format!(
            "plugin_registry length expected {}, got {}",
            expected.len(),
            registry.len()
        ));
    }

    let mut domain_owner = BTreeMap::new();
    for (index, registration) in registry.iter().enumerate() {
        let path = format!("plugin_registry[{index}]");
        let Some(expected_registration) = expected.get(index) else {
            continue;
        };
        validate_surface(
            &path,
            registration,
            expected_registration.surface.as_contract_str(),
            errors,
        );
        validate_owner_package(
            &path,
            registration,
            expected_registration.owner_package,
            errors,
        );
        validate_domains(
            &path,
            registration,
            expected_registration.domains,
            &mut domain_owner,
            errors,
        );
        validate_metadata(
            &path,
            registration,
            "capabilities",
            expected_registration.capabilities,
            errors,
        );
        validate_metadata(
            &path,
            registration,
            "effects",
            expected_registration.effects,
            errors,
        );
        validate_metadata(
            &path,
            registration,
            "hooks",
            expected_registration.hooks,
            errors,
        );
    }
}

fn validate_domains(
    path: &str,
    registration: &Value,
    expected: &[&str],
    domain_owner: &mut BTreeMap<String, String>,
    errors: &mut Vec<String>,
) {
    let field_path = format!("{path}.domains");
    let Some(values) = registration["domains"].as_array() else {
        errors.push(format!("{field_path} missing"));
        return;
    };
    let owner = registration["owner_package"]
        .as_str()
        .unwrap_or("<missing>");
    let mut actual = Vec::new();
    let mut local_seen = BTreeSet::new();
    for (index, value) in values.iter().enumerate() {
        let Some(domain) = value.as_str() else {
            errors.push(format!("{field_path}[{index}] must be string"));
            continue;
        };
        actual.push(domain);
        if !local_seen.insert(domain.to_owned()) {
            errors.push(format!("{field_path}[{index}] duplicate domain `{domain}`"));
        }
        if let Some(previous_owner) = domain_owner.insert(domain.to_owned(), owner.to_owned()) {
            errors.push(format!(
                "{field_path}[{index}] duplicate domain `{domain}` already owned by `{previous_owner}`"
            ));
        }
        if !expected.contains(&domain) {
            errors.push(format!("{field_path}[{index}] unknown `{domain}`"));
        }
    }
    if actual.as_slice() != expected {
        errors.push(format!(
            "{field_path} expected {expected:?}, got {actual:?}"
        ));
    }
}
