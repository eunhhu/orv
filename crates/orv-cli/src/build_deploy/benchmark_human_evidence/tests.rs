use super::{benchmark_raw_notes_artifact_template_filled, benchmark_raw_notes_identity_matches};

#[test]
fn benchmark_raw_notes_rejects_instruction_residue_when_template_fields_are_filled() {
    // Given: raw participant notes whose fields are filled but template prose remains.
    let content = r#"# Shop Benchmark Participant Notes

Copy this file for each participant, for example:

```text
deploy/evidence/participant-1.md
deploy/evidence/participant-2.md
```

Then set each `data.participant_runs[].raw_notes_artifact` entry in
`deploy/benchmark-evidence.json` to that relative path.

## Participant

- participant_id: participant-1
- run_id: run-1
- participant_profile: non_developer
- started_at: 2026-05-18T09:00:00Z
- completed_at: 2026-05-18T10:00:00Z

## Task Notes

Record timestamps, blockers, docs/help lookups, compiler/runtime errors, first
error-to-fix time, manual config edits, and confusing concepts.

## Evidence Review

- generated_artifact_edits: false
- manual_undocumented_security_steps: false
- ai_assistance_used: false
- failure_classification.primary: none
- failure_classification.notes: none

Human added one sentence but left the generated instructions above.
"#;

    // When: checking whether the artifact was filled beyond the template.
    let filled = benchmark_raw_notes_artifact_template_filled(content);

    // Then: generated instruction prose still counts as unfilled template residue.
    assert!(!filled);
}

#[test]
fn benchmark_raw_notes_accepts_custom_human_notes_with_ids_timestamps_and_failure_classification() {
    // Given: raw participant notes with human-specific run details and no template prose.
    let content = r#"# Shop Benchmark Participant Notes

## Participant

- participant_id: participant-1
- run_id: run-1
- participant_profile: non_developer
- started_at: 2026-05-18T09:00:00Z
- completed_at: 2026-05-18T10:00:00Z

## Task Notes

Opened the generated storefront, created product sku-42, signed in as member
carla@example.test, completed checkout, and verified shipment row sh-7001 in the
admin shipments table.

## Evidence Review

- generated_artifact_edits: false
- manual_undocumented_security_steps: false
- ai_assistance_used: false
- failure_classification.primary: none
- failure_classification.notes: completed without failure
"#;

    // When: checking whether the artifact was filled beyond the template.
    let filled = benchmark_raw_notes_artifact_template_filled(content);

    // Then: ordinary human notes pass the template-filled gate.
    assert!(filled);
}

#[test]
fn benchmark_raw_notes_rejects_empty_task_notes_section() {
    // Given: raw participant notes with filled metadata but no participant observations.
    let content = r#"# Shop Benchmark Participant Notes

## Participant

- participant_id: participant-1
- run_id: run-1
- participant_profile: non_developer
- started_at: 2026-05-18T09:00:00Z
- completed_at: 2026-05-18T10:00:00Z

## Task Notes

## Evidence Review

- generated_artifact_edits: false
- manual_undocumented_security_steps: false
- ai_assistance_used: false
- failure_classification.primary: none
- failure_classification.notes: completed without failure
"#;

    // When: checking whether the artifact was filled beyond metadata.
    let filled = benchmark_raw_notes_artifact_template_filled(content);

    // Then: per-participant raw notes require actual task observations.
    assert!(!filled);
}

#[test]
fn benchmark_raw_notes_rejects_duplicate_identity_fields() {
    // Given: raw notes where the first identity pair matches but later rows drift.
    let content = r#"# Shop Benchmark Participant Notes

## Participant

- participant_id: participant-1
- run_id: run-1
- participant_id: participant-2
- run_id: run-2
- started_at: 2026-05-18T09:00:00Z
- completed_at: 2026-05-18T10:00:00Z

## Task Notes

Completed the shop flow and retained real observations.
"#;

    // When: matching the raw notes identity against the evidence row.
    let matches =
        benchmark_raw_notes_identity_matches(content, Some("participant-1"), Some("run-1"));

    // Then: duplicate identity fields are ambiguous and must not pass.
    assert!(!matches);
}
