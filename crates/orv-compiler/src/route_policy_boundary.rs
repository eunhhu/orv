pub const POLICY_SURFACE_FIRST_PARTY_COMPILER_PLUGIN: &str = "first_party_compiler_plugin";
pub const POLICY_SURFACE_SHOP_TEMPLATE: &str = "shop_template";
pub const POLICY_SURFACE_PROVIDER_PACKAGE_TEMPLATE: &str = "provider_package_template";

#[derive(Clone, Copy)]
pub struct DefaultRouteRateLimit {
    pub limit: u32,
    pub window_seconds: u32,
    pub surface: &'static str,
}

#[derive(Clone, Copy)]
struct DefaultRouteRateLimitRule {
    method: &'static str,
    path: &'static str,
    policy: DefaultRouteRateLimit,
}

const DEFAULT_ROUTE_RATE_LIMITS: &[DefaultRouteRateLimitRule] = &[
    DefaultRouteRateLimitRule {
        method: "POST",
        path: "/members/login",
        policy: DefaultRouteRateLimit {
            limit: 10,
            window_seconds: 60,
            surface: POLICY_SURFACE_SHOP_TEMPLATE,
        },
    },
    DefaultRouteRateLimitRule {
        method: "POST",
        path: "/checkout",
        policy: DefaultRouteRateLimit {
            limit: 10,
            window_seconds: 60,
            surface: POLICY_SURFACE_SHOP_TEMPLATE,
        },
    },
    DefaultRouteRateLimitRule {
        method: "POST",
        path: "/webhooks/stripe",
        policy: DefaultRouteRateLimit {
            limit: 60,
            window_seconds: 60,
            surface: POLICY_SURFACE_PROVIDER_PACKAGE_TEMPLATE,
        },
    },
];

pub fn default_route_rate_limit(method: &str, path: &str) -> Option<DefaultRouteRateLimit> {
    DEFAULT_ROUTE_RATE_LIMITS
        .iter()
        .find(|rule| rule.method == method && rule.path == path)
        .map(|rule| rule.policy)
}

#[cfg(test)]
mod tests {
    use super::{
        default_route_rate_limit, DEFAULT_ROUTE_RATE_LIMITS,
        POLICY_SURFACE_FIRST_PARTY_COMPILER_PLUGIN, POLICY_SURFACE_PROVIDER_PACKAGE_TEMPLATE,
        POLICY_SURFACE_SHOP_TEMPLATE,
    };

    #[test]
    fn default_rate_limits_label_shop_and_provider_template_surfaces() {
        let login = default_route_rate_limit("POST", "/members/login").expect("login default");
        let checkout = default_route_rate_limit("POST", "/checkout").expect("checkout default");
        let stripe = default_route_rate_limit("POST", "/webhooks/stripe").expect("stripe default");

        assert_eq!(login.surface, POLICY_SURFACE_SHOP_TEMPLATE);
        assert_eq!(checkout.surface, POLICY_SURFACE_SHOP_TEMPLATE);
        assert_eq!(stripe.surface, POLICY_SURFACE_PROVIDER_PACKAGE_TEMPLATE);
    }

    #[test]
    fn provider_named_defaults_never_claim_first_party_compiler_plugin_surface() {
        for rule in DEFAULT_ROUTE_RATE_LIMITS {
            if rule.path.contains("stripe") || rule.path.contains("carrier") {
                assert_ne!(
                    rule.policy.surface,
                    POLICY_SURFACE_FIRST_PARTY_COMPILER_PLUGIN
                );
                assert_eq!(
                    rule.policy.surface,
                    POLICY_SURFACE_PROVIDER_PACKAGE_TEMPLATE
                );
            }
        }
    }

    #[test]
    fn unlisted_provider_like_routes_do_not_receive_template_defaults() {
        assert!(default_route_rate_limit("POST", "/webhooks/carrier").is_none());
        assert!(default_route_rate_limit("POST", "/webhooks/payment").is_none());
        assert!(default_route_rate_limit("POST", "/webhooks/shipping").is_none());
        assert!(default_route_rate_limit("POST", "/checkout/stripe").is_none());
        assert!(default_route_rate_limit("POST", "/payments").is_none());
    }
}
