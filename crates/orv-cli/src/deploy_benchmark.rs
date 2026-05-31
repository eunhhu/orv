pub const SMOKE_REQUIRED_MARKERS: &[&str] = &[
    "pass_marker",
    "build_dir",
    "base_url",
    "graph_contract",
    "dap_summary",
    "dap_source_bundle",
    "server_routes",
    "trace_stream_requested",
];

pub const FAILURE_CLASSIFICATION_CATEGORIES: &[&str] = &[
    "syntax",
    "scaffold",
    "compiler_runtime_error",
    "editor",
    "documentation",
    "deploy_config",
    "smoke_contract",
    "other",
];

pub const RECOMMENDED_PARTICIPANT_MINIMUM: u64 = 2;
pub const RECOMMENDED_PARTICIPANT_TARGET: u64 = 3;
pub const PARTICIPANT_PROFILE_NON_DEVELOPER: &str = "non_developer";

pub fn evidence_task_entries_value() -> serde_json::Value {
    serde_json::json!([
        {"task": "Project creation and first run", "target_minutes": 15, "elapsed_minutes": null, "status": "not_recorded", "notes": ""},
        {"task": "First page/theme edit", "target_minutes": 30, "elapsed_minutes": null, "status": "not_recorded", "notes": ""},
        {"task": "Product data entry", "target_minutes": 30, "elapsed_minutes": null, "status": "not_recorded", "notes": ""},
        {"task": "Product field addition", "target_minutes": 45, "elapsed_minutes": null, "status": "not_recorded", "notes": ""},
        {"task": "Form validation update", "target_minutes": 45, "elapsed_minutes": null, "status": "not_recorded", "notes": ""},
        {"task": "Auth/member flow check", "target_minutes": 30, "elapsed_minutes": null, "status": "not_recorded", "notes": ""},
        {"task": "Checkout/payment/shipping config", "target_minutes": 60, "elapsed_minutes": null, "status": "not_recorded", "notes": ""},
        {"task": "Admin verification", "target_minutes": 30, "elapsed_minutes": null, "status": "not_recorded", "notes": ""},
        {"task": "Prod build and env check", "target_minutes": 30, "elapsed_minutes": null, "status": "not_recorded", "notes": ""},
        {"task": "Smoke-test and issue fixing", "target_minutes": 45, "elapsed_minutes": null, "status": "not_recorded", "notes": ""},
    ])
}

pub fn evidence_data_value() -> serde_json::Value {
    serde_json::json!({
        "elapsed_time_per_task": "task_entries[*].elapsed_minutes",
        "docs_help_lookups": null,
        "compiler_runtime_errors": null,
        "first_error_to_fix_minutes": null,
        "ai_assistance_used": null,
        "generated_artifact_edits": null,
        "manual_undocumented_security_steps": null,
        "manual_config_edits": [],
        "smoke_test_output": null,
        "smoke_test_required_markers": smoke_required_markers_value(),
        "recommended_participant_count": {
            "minimum": RECOMMENDED_PARTICIPANT_MINIMUM,
            "target": RECOMMENDED_PARTICIPANT_TARGET,
        },
        "participant_runs": [
            {
                "run_id": null,
                "participant_id": null,
                "participant_profile": PARTICIPANT_PROFILE_NON_DEVELOPER,
                "status": "not_recorded",
                "started_at": null,
                "completed_at": null,
                "raw_notes_artifact": null,
            }
        ],
        "human_evidence_review": {
            "reviewer": "",
            "reviewed_at": null,
            "raw_notes_reviewed": null,
            "smoke_output_reviewed": null,
            "participant_identity_reviewed": null,
            "no_ai_assistance_confirmed": null,
            "notes": "",
        },
        "failure_classification": {
            "primary": null,
            "allowed_categories": FAILURE_CLASSIFICATION_CATEGORIES,
            "notes": "",
        },
        "participant_notes": "",
    })
}

pub fn smoke_required_markers_value() -> serde_json::Value {
    serde_json::json!(SMOKE_REQUIRED_MARKERS)
}

pub fn preflight_contract_value() -> serde_json::Value {
    serde_json::json!({
        "schema_version": 1,
        "kind": "orv.benchmark.shop_5h",
        "goal": "non-developer builds and verifies a small shop without AI assistance in under 5 hours",
        "max_elapsed_minutes": 300,
        "automated_gate": [
            "orv init my-shop --template shop",
            "orv check .",
            "orv build . --prod --out dist",
            "orv verify-build dist",
            "orv deploy-env-check dist",
            "orv run-build dist",
            "sh dist/deploy/smoke-test.sh",
            "orv benchmark-report dist --require-pass",
        ],
        "success_criteria": [
            "edit home page copy and theme tokens",
            "create 3 products",
            "add one product field and show it in catalog/admin",
            "sign up and log in as a member",
            "add an item to cart",
            "complete checkout",
            "capture mock payment",
            "book mock shipping",
            "view order/payment/shipment rows in admin",
            "run prod build",
            "pass deploy env check",
            "pass generated smoke-test",
            "reveal route/html/db-related execution output back to source through origin artifacts",
        ],
        "time_budget": [
            {"task": "Project creation and first run", "target_minutes": 15},
            {"task": "First page/theme edit", "target_minutes": 30},
            {"task": "Product data entry", "target_minutes": 30},
            {"task": "Product field addition", "target_minutes": 45},
            {"task": "Form validation update", "target_minutes": 45},
            {"task": "Auth/member flow check", "target_minutes": 30},
            {"task": "Checkout/payment/shipping config", "target_minutes": 60},
            {"task": "Admin verification", "target_minutes": 30},
            {"task": "Prod build and env check", "target_minutes": 30},
            {"task": "Smoke-test and issue fixing", "target_minutes": 45},
        ],
        "data_to_record": [
            "non-negative elapsed time and non-empty notes per recorded task",
            "non-negative number of docs/help lookups",
            "non-negative number of compiler/runtime errors",
            "non-negative time from first error to fix",
            "whether any AI assistance was used",
            "whether generated runtime/build artifacts were edited",
            "whether any required security step was manual and undocumented",
            "all manual config edits",
            "smoke-test output",
            "smoke-test required markers",
            "smoke-test DAP source-bundle marker",
            "participant run metadata",
            "human evidence reviewer attestation",
            "failure classification",
            "participant notes on confusing concepts",
        ],
    })
}
