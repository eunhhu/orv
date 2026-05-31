use orv_hir::{domain_boundary_descriptor, origin_call_domain_method, DomainSurface};

pub(crate) fn adapter_runtime_feature(call: &str) -> Option<&'static str> {
    let (domain, method) = origin_call_domain_method(call)?;
    if method != "connect" {
        return None;
    }
    let descriptor = domain_boundary_descriptor(domain);
    match (
        descriptor.surface,
        descriptor.owner_package,
        descriptor.domain,
    ) {
        (DomainSurface::FirstPartyCompilerPlugin, "orv-data", "db") => Some("db_adapter"),
        (DomainSurface::LibraryProviderPackage, "orv-commerce", "payment") => {
            Some("payment_adapter")
        }
        (DomainSurface::LibraryProviderPackage, "orv-commerce", "shipping") => {
            Some("shipping_adapter")
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::adapter_runtime_feature;

    #[test]
    fn commerce_adapter_features_require_library_provider_boundary() {
        assert_eq!(
            adapter_runtime_feature("@payment.connect"),
            Some("payment_adapter")
        );
        assert_eq!(
            adapter_runtime_feature("@shipping.connect"),
            Some("shipping_adapter")
        );
    }

    #[test]
    fn provider_names_do_not_become_core_runtime_features() {
        assert_eq!(adapter_runtime_feature("@Stripe.connect"), None);
        assert_eq!(adapter_runtime_feature("@Stripe.capture"), None);
        assert_eq!(adapter_runtime_feature("@carrier.connect"), None);
        assert_eq!(adapter_runtime_feature("@carrier.book"), None);
        assert_eq!(adapter_runtime_feature("@custom.connect"), None);
    }

    #[test]
    fn non_connect_calls_do_not_emit_adapter_features() {
        assert_eq!(adapter_runtime_feature("@payment.capture"), None);
        assert_eq!(adapter_runtime_feature("@shipping.book"), None);
        assert_eq!(adapter_runtime_feature("@db.query"), None);
    }
}
