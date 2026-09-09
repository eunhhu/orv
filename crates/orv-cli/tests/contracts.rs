//! Public CLI contracts share one harness and one integration-test executable.

mod support;

#[path = "dap_editor_source_bundle_contract/support.rs"]
mod source_bundle_support;

mod benchmark_evidence_verify_contract;
mod benchmark_prepare_handoff_contract;
mod build_artifacts_contract;
mod check_cli_contract;
mod client_bundle_contract;
mod commerce_adapters_contract;
mod commerce_provider_hardening_contract;
mod compiler_pipeline_contract;
mod compiler_plugin_boundary_contract;
mod consumer_artifact_boundary_contract;
mod core_spine_contract;
mod dap_async_transport;
mod dap_debug_contract;
mod dap_debug_nested_contract;
mod dap_editor_source_bundle_contract;
mod dap_editor_source_bundle_summary_parity_contract;
mod db_adapters_contract;
mod db_data_migration;
mod db_persistence_contract;
mod deploy_runbook_benchmark_contract;
mod deploy_schema_contract;
mod editor_snapshot_export_contract;
mod editor_trace_contract;
mod html_render_contract;
mod lsp_bootstrap_contract;
mod native_server_contract;
mod origin_header_contract;
mod origin_map_contract;
mod project_graph_contract;
mod provider_secret_redaction_contract;
mod reveal_benchmark_evidence_contract;
mod reveal_coverage_contract;
mod reveal_payload_contract;
mod runtime_cli_contract;
mod shop_acceptance_contract;
mod shop_acceptance_smoke_contract;
mod shop_security_boundary_contract;
mod shop_template_contract;
mod test_runner_contract;
