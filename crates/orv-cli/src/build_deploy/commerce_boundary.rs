use super::{deploy_provider_env, DeployProviderEnv};

pub(crate) fn deploy_commerce_adapter_surface(kind: &str) -> &'static str {
    orv_hir::domain_boundary_descriptor(kind)
        .surface
        .as_contract_str()
}

pub(crate) fn deploy_commerce_adapter_package(kind: &str) -> &'static str {
    let descriptor = orv_hir::domain_boundary_descriptor(kind);
    if descriptor.surface.is_library_provider_package() {
        descriptor.owner_package
    } else {
        "unknown"
    }
}

pub(crate) fn deploy_commerce_provider_package(provider: &str) -> Option<&'static str> {
    match provider {
        "stripe" => Some("orv-stripe"),
        "carrier" => Some("orv-carrier"),
        _ => None,
    }
}

pub(crate) fn deploy_commerce_adapter_request_value(kind: &str) -> serde_json::Value {
    let (request_kind, payload) = match kind {
        "payment" => ("payment.capture", "payment capture payload"),
        "shipping" => ("shipping.booking", "shipping booking payload"),
        _ => ("commerce.request", "commerce payload"),
    };
    serde_json::json!({
        "method": "POST",
        "content_type": "application/json",
        "kind": request_kind,
        "body": {
            "kind": request_kind,
            "payload": payload,
        },
    })
}

pub(crate) fn commerce_provider(url: &str, kind: &str) -> Option<String> {
    let (scheme, target) = url.split_once("://")?;
    if target.is_empty() {
        return None;
    }
    match (kind, scheme) {
        ("payment", "stripe") => Some("stripe".to_string()),
        ("shipping", "carrier") => Some("carrier".to_string()),
        _ => None,
    }
}

pub(crate) fn commerce_provider_env(provider: &str) -> Vec<DeployProviderEnv> {
    match provider {
        "stripe" => vec![
            deploy_provider_env("STRIPE_API_ENDPOINT", false, "api_endpoint"),
            deploy_provider_env("STRIPE_SECRET_KEY", true, "api_secret"),
            deploy_provider_env("STRIPE_WEBHOOK_SECRET", false, "webhook_signature"),
            deploy_provider_env(
                "STRIPE_WEBHOOK_SECRET_PREVIOUS",
                false,
                "webhook_signature_previous",
            ),
        ],
        "carrier" => vec![
            deploy_provider_env("CARRIER_API_ENDPOINT", false, "api_endpoint"),
            deploy_provider_env("CARRIER_API_KEY", true, "api_key"),
            deploy_provider_env("CARRIER_WEBHOOK_SECRET", false, "webhook_signature"),
        ],
        _ => Vec::new(),
    }
}

pub(crate) fn commerce_provider_env_for_url(provider: &str, url: &str) -> Vec<DeployProviderEnv> {
    if provider == "stripe" && url.starts_with("stripe://webhook") {
        return vec![
            deploy_provider_env("STRIPE_WEBHOOK_SECRET", false, "webhook_signature"),
            deploy_provider_env(
                "STRIPE_WEBHOOK_SECRET_PREVIOUS",
                false,
                "webhook_signature_previous",
            ),
        ];
    }
    commerce_provider_env(provider)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commerce_adapter_package_uses_hir_owner_descriptor() {
        assert_eq!(
            deploy_commerce_adapter_surface("payment"),
            "library_provider_package"
        );
        assert_eq!(deploy_commerce_adapter_package("payment"), "orv-commerce");
        assert_eq!(deploy_commerce_adapter_package("shipping"), "orv-commerce");
        assert_eq!(deploy_commerce_adapter_package("route"), "unknown");
    }

    #[test]
    fn commerce_provider_rejects_cross_kind_provider_scheme() {
        assert_eq!(
            commerce_provider("stripe://local", "payment"),
            Some("stripe".to_string())
        );
        assert_eq!(commerce_provider("stripe://local", "shipping"), None);
        assert_eq!(commerce_provider("carrier://local", "payment"), None);
        assert_eq!(
            commerce_provider("carrier://local", "shipping"),
            Some("carrier".to_string())
        );
        assert_eq!(commerce_provider("stripe://", "payment"), None);
    }
}
