use serde_json::Value;

pub fn validate_compiler_plugin_boundary_inventory(inventory: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let Some(descriptors) = inventory["domain_descriptors"].as_array() else {
        return vec!["domain_descriptors missing".to_string()];
    };
    for (index, descriptor) in descriptors.iter().enumerate() {
        validate_domain_descriptor(
            &format!("domain_descriptors[{index}]"),
            descriptor,
            &mut errors,
        );
    }
    errors
}

pub fn assert_inventory_rejection_contains(inventory: &Value, expected: &str) {
    let errors = validate_compiler_plugin_boundary_inventory(inventory);
    assert!(
        errors.iter().any(|error| error.contains(expected)),
        "expected error containing {expected:?}, got {errors:?}"
    );
}

pub fn domain_descriptor_index(inventory: &Value, domain: &str) -> usize {
    inventory["domain_descriptors"]
        .as_array()
        .expect("domain descriptors")
        .iter()
        .position(|descriptor| descriptor["domain"].as_str() == Some(domain))
        .expect("domain descriptor")
}

fn validate_domain_descriptor(path: &str, descriptor: &Value, errors: &mut Vec<String>) {
    let Some(domain) = descriptor["domain"].as_str() else {
        errors.push(format!("{path}.domain missing"));
        return;
    };
    let expected = orv_hir::domain_boundary_descriptor(domain);
    validate_surface(path, descriptor, expected.surface.as_contract_str(), errors);
    validate_owner_package(path, descriptor, expected.owner_package, errors);
    validate_metadata(
        path,
        descriptor,
        "capabilities",
        expected.capabilities,
        errors,
    );
    validate_metadata(path, descriptor, "effects", expected.effects, errors);
    validate_metadata(path, descriptor, "hooks", expected.hooks, errors);
}

fn validate_surface(path: &str, descriptor: &Value, expected: &str, errors: &mut Vec<String>) {
    let Some(surface) = descriptor["surface"].as_str() else {
        errors.push(format!("{path}.surface missing"));
        return;
    };
    if orv_hir::DomainSurface::from_contract_str(surface).is_none() {
        errors.push(format!("{path}.surface unknown `{surface}`"));
        return;
    }
    if surface != expected {
        errors.push(format!(
            "{path}.surface expected `{expected}`, got `{surface}`"
        ));
    }
}

fn validate_owner_package(
    path: &str,
    descriptor: &Value,
    expected: &str,
    errors: &mut Vec<String>,
) {
    let Some(owner_package) = descriptor["owner_package"]
        .as_str()
        .filter(|value| !value.is_empty())
    else {
        errors.push(format!("{path}.owner_package missing"));
        return;
    };
    if owner_package != expected {
        errors.push(format!(
            "{path}.owner_package expected `{expected}`, got `{owner_package}`"
        ));
    }
}

fn validate_metadata(
    path: &str,
    descriptor: &Value,
    field: &str,
    expected: &[&str],
    errors: &mut Vec<String>,
) {
    let field_path = format!("{path}.{field}");
    let Some(values) = descriptor[field].as_array() else {
        errors.push(format!("{field_path} missing"));
        return;
    };
    let mut actual = Vec::new();
    for (index, value) in values.iter().enumerate() {
        let Some(text) = value.as_str() else {
            errors.push(format!("{field_path}[{index}] must be string"));
            continue;
        };
        actual.push(text);
        if !expected.contains(&text) {
            errors.push(format!("{field_path}[{index}] unknown `{text}`"));
        }
    }
    if actual.is_empty() && !expected.is_empty() {
        errors.push(format!("{field_path} empty; expected {expected:?}"));
    }
    if actual.as_slice() != expected {
        errors.push(format!(
            "{field_path} expected {expected:?}, got {actual:?}"
        ));
    }
}
