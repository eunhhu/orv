#![allow(clippy::redundant_pub_crate, clippy::wildcard_imports)]

use super::*;

pub(crate) fn cmd_lsp_snapshot(path: &Path) -> anyhow::Result<()> {
    let entry = project_entry_path(path)?;
    let value = lsp_snapshot_json(&entry)?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

pub(crate) fn cmd_lsp_reveal(dir: &Path, origin_id: &str) -> anyhow::Result<()> {
    let value = lsp_reveal_json(dir, origin_id)?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

pub(crate) fn cmd_lsp_serve(use_stdio: bool) -> anyhow::Result<()> {
    if !use_stdio {
        anyhow::bail!("lsp serve currently requires --stdio");
    }
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();
    lsp_serve_stdio_stream(&mut reader, &mut writer)
}

#[cfg(test)]
pub(crate) fn lsp_jsonrpc_response(request: &serde_json::Value) -> serde_json::Value {
    LspSession::default().jsonrpc_response(request)
}

#[derive(Default)]
pub(crate) struct LspSession {
    pub(crate) open_documents: HashMap<PathBuf, String>,
    pub(crate) workspace_root: Option<PathBuf>,
}

impl LspSession {
    pub(crate) fn message_response(
        &mut self,
        request: &serde_json::Value,
    ) -> Option<serde_json::Value> {
        if request.get("id").is_none() {
            self.handle_notification(request);
            return None;
        }
        Some(self.jsonrpc_response(request))
    }

    pub(crate) fn jsonrpc_response(&mut self, request: &serde_json::Value) -> serde_json::Value {
        let id = request
            .get("id")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        match request.get("method").and_then(serde_json::Value::as_str) {
            Some("initialize") => self.initialize_response(request, &id),
            Some("shutdown") => serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": serde_json::Value::Null,
            }),
            Some("textDocument/documentSymbol") => {
                lsp_jsonrpc_result_or_invalid_params(&id, self.document_symbol_result(request))
            }
            Some("textDocument/codeLens") => {
                lsp_jsonrpc_result_or_invalid_params(&id, self.code_lens_result(request))
            }
            Some("textDocument/codeAction") => {
                lsp_jsonrpc_result_or_invalid_params(&id, self.code_action_result(request))
            }
            Some("textDocument/documentLink") => {
                lsp_jsonrpc_result_or_invalid_params(&id, self.document_link_result(request))
            }
            Some("textDocument/foldingRange") => {
                lsp_jsonrpc_result_or_invalid_params(&id, self.folding_range_result(request))
            }
            Some("textDocument/selectionRange") => {
                lsp_jsonrpc_result_or_invalid_params(&id, self.selection_range_result(request))
            }
            Some("textDocument/semanticTokens/full") => {
                lsp_jsonrpc_result_or_invalid_params(&id, self.semantic_tokens_result(request))
            }
            Some("textDocument/diagnostic") => lsp_jsonrpc_result_or_invalid_params(
                &id,
                self.text_document_diagnostic_result(request),
            ),
            Some("workspace/diagnostic") => {
                lsp_jsonrpc_result_or_invalid_params(&id, self.workspace_diagnostic_result())
            }
            Some("workspace/executeCommand") => {
                lsp_jsonrpc_result_or_invalid_params(&id, self.execute_command_result(request))
            }
            Some(
                method @ ("textDocument/definition"
                | "textDocument/declaration"
                | "textDocument/implementation"
                | "textDocument/typeDefinition"
                | "textDocument/moniker"),
            ) => lsp_jsonrpc_result_or_invalid_params(&id, self.navigation_result(method, request)),
            Some(
                method @ ("textDocument/prepareCallHierarchy"
                | "textDocument/prepareTypeHierarchy"
                | "callHierarchy/outgoingCalls"
                | "callHierarchy/incomingCalls"
                | "typeHierarchy/supertypes"
                | "typeHierarchy/subtypes"),
            ) => lsp_jsonrpc_result_or_invalid_params(&id, self.hierarchy_result(method, request)),
            Some(method @ ("textDocument/documentColor" | "textDocument/colorPresentation")) => {
                lsp_jsonrpc_result_or_invalid_params(&id, self.color_result(method, request))
            }
            Some("textDocument/linkedEditingRange") => {
                lsp_jsonrpc_result_or_invalid_params(&id, self.linked_editing_range_result(request))
            }
            Some("textDocument/references") => {
                lsp_jsonrpc_result_or_invalid_params(&id, self.references_result(request))
            }
            Some("textDocument/documentHighlight") => {
                lsp_jsonrpc_result_or_invalid_params(&id, self.document_highlight_result(request))
            }
            Some("textDocument/prepareRename") => {
                lsp_jsonrpc_result_or_invalid_params(&id, self.prepare_rename_result(request))
            }
            Some("textDocument/rename") => {
                lsp_jsonrpc_result_or_invalid_params(&id, self.rename_result(request))
            }
            Some("textDocument/hover") => {
                lsp_jsonrpc_result_or_invalid_params(&id, self.hover_result(request))
            }
            Some("textDocument/signatureHelp") => {
                lsp_jsonrpc_result_or_invalid_params(&id, self.signature_help_result(request))
            }
            Some("textDocument/inlayHint") => {
                lsp_jsonrpc_result_or_invalid_params(&id, self.inlay_hint_result(request))
            }
            Some(
                method @ ("textDocument/formatting"
                | "textDocument/rangeFormatting"
                | "textDocument/onTypeFormatting"),
            ) => lsp_jsonrpc_result_or_invalid_params(&id, self.formatting_result(method, request)),
            Some("textDocument/completion") => {
                lsp_jsonrpc_result_or_invalid_params(&id, self.completion_result(request))
            }
            Some("workspace/symbol") => {
                lsp_jsonrpc_result_or_invalid_params(&id, self.workspace_symbol_result(request))
            }
            Some(method) => lsp_jsonrpc_method_not_found(&id, method),
            None => lsp_jsonrpc_error(&id, -32600, "invalid request"),
        }
    }

    fn color_result(
        &self,
        method: &str,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        match method {
            "textDocument/documentColor" => self.document_color_result(request),
            "textDocument/colorPresentation" => Self::color_presentation_result(request),
            _ => unreachable!("color method dispatch is filtered by jsonrpc_response"),
        }
    }

    fn formatting_result(
        &self,
        method: &str,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        match method {
            "textDocument/formatting" => self.document_formatting_result(request),
            "textDocument/rangeFormatting" => self.range_formatting_result(request),
            "textDocument/onTypeFormatting" => self.on_type_formatting_result(request),
            _ => unreachable!("formatting method dispatch is filtered by jsonrpc_response"),
        }
    }

    fn navigation_result(
        &self,
        method: &str,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        match method {
            "textDocument/definition"
            | "textDocument/declaration"
            | "textDocument/implementation" => self.definition_result(request),
            "textDocument/typeDefinition" => self.type_definition_result(request),
            "textDocument/moniker" => self.moniker_result(request),
            _ => unreachable!("navigation method dispatch is filtered by jsonrpc_response"),
        }
    }

    fn hierarchy_result(
        &self,
        method: &str,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        match method {
            "textDocument/prepareCallHierarchy" => self.prepare_call_hierarchy_result(request),
            "textDocument/prepareTypeHierarchy" => self.prepare_type_hierarchy_result(request),
            "callHierarchy/outgoingCalls" => self.call_hierarchy_outgoing_result(request),
            "callHierarchy/incomingCalls" => self.call_hierarchy_incoming_result(request),
            "typeHierarchy/supertypes" | "typeHierarchy/subtypes" => {
                Self::empty_type_hierarchy_result(request)
            }
            _ => unreachable!("hierarchy method dispatch is filtered by jsonrpc_response"),
        }
    }

    fn initialize_response(
        &mut self,
        request: &serde_json::Value,
        id: &serde_json::Value,
    ) -> serde_json::Value {
        self.handle_initialize(request);
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "serverInfo": {
                    "name": "orv-lsp",
                    "version": env!("CARGO_PKG_VERSION"),
                },
                "capabilities": {
                    "textDocumentSync": {
                        "openClose": true,
                        "change": 1,
                        "save": {
                            "includeText": true,
                        },
                    },
                    "documentSymbolProvider": true,
                    "codeLensProvider": {
                        "resolveProvider": false,
                    },
                    "codeActionProvider": {
                        "codeActionKinds": ["quickfix"],
                    },
                    "executeCommandProvider": {
                        "commands": ["orv.revealSourceNode", "orv.revealDiagnostic"],
                    },
                    "documentLinkProvider": {
                        "resolveProvider": false,
                    },
                    "foldingRangeProvider": true,
                    "selectionRangeProvider": true,
                    "semanticTokensProvider": {
                        "legend": {
                            "tokenTypes": ["namespace", "type", "function"],
                            "tokenModifiers": ["declaration"],
                        },
                        "full": true,
                        "range": false,
                    },
                    "workspaceSymbolProvider": true,
                    "definitionProvider": true,
                    "declarationProvider": true,
                    "typeDefinitionProvider": true,
                    "implementationProvider": true,
                    "monikerProvider": true,
                    "callHierarchyProvider": true,
                    "typeHierarchyProvider": true,
                    "colorProvider": true,
                    "linkedEditingRangeProvider": true,
                    "referencesProvider": true,
                    "documentHighlightProvider": true,
                    "renameProvider": {
                        "prepareProvider": true,
                    },
                    "hoverProvider": true,
                    "signatureHelpProvider": {
                        "triggerCharacters": ["(", ","],
                    },
                    "inlayHintProvider": true,
                    "documentFormattingProvider": true,
                    "documentRangeFormattingProvider": true,
                    "documentOnTypeFormattingProvider": {
                        "firstTriggerCharacter": "}",
                        "moreTriggerCharacter": ["{", "\n"],
                    },
                    "completionProvider": {
                        "triggerCharacters": ["@", ".", ":"],
                    },
                    "diagnosticProvider": {
                        "interFileDependencies": true,
                        "workspaceDiagnostics": true,
                    },
                },
            },
        })
    }

    fn handle_initialize(&mut self, request: &serde_json::Value) {
        let Some(root_uri) = request
            .pointer("/params/rootUri")
            .and_then(serde_json::Value::as_str)
        else {
            return;
        };
        if let Ok(path) = lsp_file_uri_path(root_uri) {
            self.workspace_root = Some(path);
        }
    }

    fn text_document_diagnostic_result(
        &self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let uri = lsp_text_document_uri(request)?;
        let path = lsp_file_uri_path(uri)?;
        let loaded = self.loaded_project_for_text_document(&path)?;
        let diagnostics = lsp_project_diagnostics(&loaded);
        let diagnostics = lsp_source_file_for_path(&loaded.files, &path)
            .map_or_else(Vec::new, |file| {
                lsp_diagnostics_json_for_file(&diagnostics, &loaded.files, file.id)
            });
        Ok(serde_json::json!({
            "kind": "full",
            "items": diagnostics,
        }))
    }

    fn workspace_diagnostic_result(&self) -> anyhow::Result<serde_json::Value> {
        let root = self.workspace_root.as_ref().ok_or_else(|| {
            anyhow::anyhow!("initialize.params.rootUri is required before workspace/diagnostic")
        })?;
        let entry = project_entry_path(root)?;
        let loaded = self.loaded_project_for_path(&entry)?;
        Ok(serde_json::json!({
            "items": lsp_workspace_diagnostic_items_json(&loaded),
        }))
    }

    fn execute_command_result(
        &self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let command = request
            .pointer("/params/command")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("command must be a string"))?;
        match command {
            "orv.revealSourceNode" => self.execute_reveal_source_node(request),
            "orv.revealDiagnostic" => Ok(lsp_execute_reveal_diagnostic_json(request)),
            _ => Err(anyhow::anyhow!("unsupported LSP command `{command}`")),
        }
    }

    fn execute_reveal_source_node(
        &self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let node_id = request
            .pointer("/params/arguments/0")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| anyhow::anyhow!("orv.revealSourceNode requires source node id"))?;
        let node_id = ProjectNodeId::try_from(node_id)
            .map_err(|_| anyhow::anyhow!("source node id is too large"))?;
        let root = self.workspace_root.as_ref().ok_or_else(|| {
            anyhow::anyhow!("initialize.params.rootUri is required before workspace/executeCommand")
        })?;
        let entry = project_entry_path(root)?;
        let loaded = self.loaded_project_for_path(&entry)?;
        let node = loaded
            .graph
            .nodes
            .iter()
            .find(|node| node.id == node_id)
            .ok_or_else(|| anyhow::anyhow!("unknown source node `{node_id}`"))?;
        Ok(serde_json::json!({
            "command": "orv.revealSourceNode",
            "source_node": node.id,
            "name": node.name,
            "kind": lsp_symbol_kind(node.kind).unwrap_or("Symbol"),
            "location": lsp_location_json(node, &loaded.files),
        }))
    }

    fn definition_result(&self, request: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let uri = lsp_text_document_uri(request)?;
        let path = lsp_file_uri_path(uri)?;
        let position = lsp_text_document_position(request)?;
        let loaded = self.loaded_project_for_path(&path)?;
        let Some(file) = lsp_source_file_for_path(&loaded.files, &path) else {
            return Ok(serde_json::Value::Null);
        };
        let byte = lsp_position_to_byte(&file.source, position);
        let Some(name) = identifier_at_byte(&file.source, byte) else {
            return Ok(serde_json::Value::Null);
        };
        let Some(node) = lsp_definition_node(&loaded.graph, name) else {
            return Ok(serde_json::Value::Null);
        };
        Ok(lsp_location_json(node, &loaded.files))
    }

    fn type_definition_result(
        &self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let uri = lsp_text_document_uri(request)?;
        let path = lsp_file_uri_path(uri)?;
        let position = lsp_text_document_position(request)?;
        let loaded = self.loaded_project_for_path(&path)?;
        let Some(file) = lsp_source_file_for_path(&loaded.files, &path) else {
            return Ok(serde_json::Value::Null);
        };
        let byte = lsp_position_to_byte(&file.source, position);
        let Some(name) = identifier_at_byte(&file.source, byte) else {
            return Ok(serde_json::Value::Null);
        };
        let Some(node) = lsp_type_definition_node(&loaded.graph, name) else {
            return Ok(serde_json::Value::Null);
        };
        Ok(lsp_location_json(node, &loaded.files))
    }

    fn document_color_result(
        &self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let uri = lsp_text_document_uri(request)?;
        let path = lsp_file_uri_path(uri)?;
        let loaded = self.loaded_project_for_path(&path)?;
        let Some(file) = lsp_source_file_for_path(&loaded.files, &path) else {
            return Ok(serde_json::Value::Array(Vec::new()));
        };
        Ok(serde_json::Value::Array(lsp_document_colors_json(
            &file.source,
        )))
    }

    fn color_presentation_result(request: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let _uri = lsp_text_document_uri(request)?;
        let range = request
            .pointer("/params/range")
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("range must be an object"))?;
        let (red, green, blue, alpha) = lsp_color_param(request)?;
        let label = lsp_hex_color_label(red, green, blue, alpha);
        Ok(serde_json::Value::Array(vec![serde_json::json!({
            "label": label,
            "textEdit": {
                "range": range,
                "newText": label,
            },
        })]))
    }

    fn moniker_result(&self, request: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let uri = lsp_text_document_uri(request)?;
        let path = lsp_file_uri_path(uri)?;
        let position = lsp_text_document_position(request)?;
        let loaded = self.loaded_project_for_path(&path)?;
        let Some(file) = lsp_source_file_for_path(&loaded.files, &path) else {
            return Ok(serde_json::Value::Null);
        };
        let byte = lsp_position_to_byte(&file.source, position);
        let Some(name) = identifier_at_byte(&file.source, byte) else {
            return Ok(serde_json::Value::Null);
        };
        let Some(node) = lsp_definition_node(&loaded.graph, name) else {
            return Ok(serde_json::Value::Null);
        };
        Ok(serde_json::Value::Array(vec![lsp_moniker_json(node)]))
    }

    fn prepare_call_hierarchy_result(
        &self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let uri = lsp_text_document_uri(request)?;
        let path = lsp_file_uri_path(uri)?;
        let position = lsp_text_document_position(request)?;
        let loaded = self.loaded_project_for_path(&path)?;
        let Some(file) = lsp_source_file_for_path(&loaded.files, &path) else {
            return Ok(serde_json::Value::Null);
        };
        let byte = lsp_position_to_byte(&file.source, position);
        let Some(name) = identifier_at_byte(&file.source, byte) else {
            return Ok(serde_json::Value::Null);
        };
        let Some(function) = lsp_function_stmt_by_name(&loaded.program, name) else {
            return Ok(serde_json::Value::Null);
        };
        Ok(serde_json::Value::Array(vec![
            lsp_call_hierarchy_item_json(function, &loaded.files),
        ]))
    }

    fn prepare_type_hierarchy_result(
        &self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let uri = lsp_text_document_uri(request)?;
        let path = lsp_file_uri_path(uri)?;
        let position = lsp_text_document_position(request)?;
        let loaded = self.loaded_project_for_path(&path)?;
        let Some(file) = lsp_source_file_for_path(&loaded.files, &path) else {
            return Ok(serde_json::Value::Null);
        };
        let byte = lsp_position_to_byte(&file.source, position);
        let Some(name) = identifier_at_byte(&file.source, byte) else {
            return Ok(serde_json::Value::Null);
        };
        let Some(node) = lsp_type_definition_node(&loaded.graph, name) else {
            return Ok(serde_json::Value::Null);
        };
        Ok(serde_json::Value::Array(vec![
            lsp_type_hierarchy_item_json(node, &loaded.files),
        ]))
    }

    fn empty_type_hierarchy_result(
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        request
            .pointer("/params/item/name")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("typeHierarchy item.name must be a string"))?;
        Ok(serde_json::Value::Array(Vec::new()))
    }

    fn call_hierarchy_outgoing_result(
        &self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let (loaded, caller_name) = self.loaded_project_for_call_hierarchy_item(request)?;
        let Some(caller) = lsp_function_stmt_by_name(&loaded.program, caller_name) else {
            return Ok(serde_json::Value::Array(Vec::new()));
        };
        Ok(serde_json::Value::Array(lsp_call_hierarchy_outgoing_calls(
            caller,
            &loaded.program,
            &loaded.files,
        )))
    }

    fn call_hierarchy_incoming_result(
        &self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let (loaded, callee_name) = self.loaded_project_for_call_hierarchy_item(request)?;
        Ok(serde_json::Value::Array(lsp_call_hierarchy_incoming_calls(
            callee_name,
            &loaded.program,
            &loaded.files,
        )))
    }

    fn loaded_project_for_call_hierarchy_item<'a>(
        &self,
        request: &'a serde_json::Value,
    ) -> anyhow::Result<(orv_project::LoadedProject, &'a str)> {
        let name = request
            .pointer("/params/item/name")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("callHierarchy item.name must be a string"))?;
        let uri = request
            .pointer("/params/item/uri")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("callHierarchy item.uri must be a string"))?;
        let path = lsp_file_uri_path(uri)?;
        Ok((self.loaded_project_for_path(&path)?, name))
    }

    fn linked_editing_range_result(
        &self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let uri = lsp_text_document_uri(request)?;
        let path = lsp_file_uri_path(uri)?;
        let position = lsp_text_document_position(request)?;
        let loaded = self.loaded_project_for_path(&path)?;
        let Some(file) = lsp_source_file_for_path(&loaded.files, &path) else {
            return Ok(serde_json::Value::Null);
        };
        let byte = lsp_position_to_byte(&file.source, position);
        let Some((_, _, name)) = lsp_renamable_identifier_span_at_byte(&file.source, byte) else {
            return Ok(serde_json::Value::Null);
        };
        Ok(lsp_linked_editing_range_json(&file.source, name))
    }

    fn references_result(&self, request: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let uri = lsp_text_document_uri(request)?;
        let path = lsp_file_uri_path(uri)?;
        let position = lsp_text_document_position(request)?;
        let loaded = self.loaded_project_for_path(&path)?;
        let Some(file) = lsp_source_file_for_path(&loaded.files, &path) else {
            return Ok(serde_json::Value::Array(Vec::new()));
        };
        let byte = lsp_position_to_byte(&file.source, position);
        if let Some(field) = lsp_domain_field_at_byte(&file.source, byte) {
            return Ok(serde_json::Value::Array(
                lsp_domain_field_reference_locations_json(&loaded.files, field.kind, field.name),
            ));
        }
        let Some((_, _, name)) = lsp_renamable_identifier_span_at_byte(&file.source, byte) else {
            return Ok(serde_json::Value::Array(Vec::new()));
        };
        Ok(serde_json::Value::Array(lsp_reference_locations_json(
            &loaded.files,
            name,
        )))
    }

    fn document_highlight_result(
        &self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let uri = lsp_text_document_uri(request)?;
        let path = lsp_file_uri_path(uri)?;
        let position = lsp_text_document_position(request)?;
        let loaded = self.loaded_project_for_path(&path)?;
        let Some(file) = lsp_source_file_for_path(&loaded.files, &path) else {
            return Ok(serde_json::Value::Array(Vec::new()));
        };
        let byte = lsp_position_to_byte(&file.source, position);
        if let Some(field) = lsp_domain_field_at_byte(&file.source, byte) {
            return Ok(serde_json::Value::Array(
                lsp_domain_field_occurrences(&file.source, field.kind, field.name)
                    .into_iter()
                    .map(|(start, end)| {
                        serde_json::json!({
                            "range": lsp_range_for_source(
                                &file.source,
                                u32::try_from(start).unwrap_or(u32::MAX),
                                u32::try_from(end).unwrap_or(u32::MAX),
                            ),
                            "kind": 1,
                        })
                    })
                    .collect(),
            ));
        }
        let Some((_, _, name)) = lsp_renamable_identifier_span_at_byte(&file.source, byte) else {
            return Ok(serde_json::Value::Array(Vec::new()));
        };
        Ok(serde_json::Value::Array(
            identifier_occurrences(&file.source, name)
                .into_iter()
                .map(|(start, end)| {
                    serde_json::json!({
                        "range": lsp_range_for_source(
                            &file.source,
                            u32::try_from(start).unwrap_or(u32::MAX),
                            u32::try_from(end).unwrap_or(u32::MAX),
                        ),
                        "kind": 1,
                    })
                })
                .collect(),
        ))
    }

    fn prepare_rename_result(
        &self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let uri = lsp_text_document_uri(request)?;
        let path = lsp_file_uri_path(uri)?;
        let position = lsp_text_document_position(request)?;
        let loaded = self.loaded_project_for_path(&path)?;
        let Some(file) = lsp_source_file_for_path(&loaded.files, &path) else {
            return Ok(serde_json::Value::Null);
        };
        let byte = lsp_position_to_byte(&file.source, position);
        let Some((start, end, name)) = lsp_renamable_identifier_span_at_byte(&file.source, byte)
        else {
            return Ok(serde_json::Value::Null);
        };
        Ok(serde_json::json!({
            "range": lsp_range_for_source(
                &file.source,
                u32::try_from(start).unwrap_or(u32::MAX),
                u32::try_from(end).unwrap_or(u32::MAX),
            ),
            "placeholder": name,
        }))
    }

    fn rename_result(&self, request: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let uri = lsp_text_document_uri(request)?;
        let path = lsp_file_uri_path(uri)?;
        let position = lsp_text_document_position(request)?;
        let new_name = request
            .pointer("/params/newName")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("newName must be a string"))?;
        if !lsp_renamable_identifier_name(new_name) {
            return Err(anyhow::anyhow!(
                "newName must be a valid non-keyword identifier"
            ));
        }
        let loaded = self.loaded_project_for_path(&path)?;
        let Some(file) = lsp_source_file_for_path(&loaded.files, &path) else {
            return Ok(serde_json::json!({ "changes": {} }));
        };
        let byte = lsp_position_to_byte(&file.source, position);
        let Some((_, _, name)) = lsp_renamable_identifier_span_at_byte(&file.source, byte) else {
            return Ok(serde_json::json!({ "changes": {} }));
        };
        let mut changes = serde_json::Map::new();
        for file in &loaded.files {
            let edits: Vec<_> = identifier_occurrences(&file.source, name)
                .into_iter()
                .map(|(start, end)| {
                    serde_json::json!({
                        "range": lsp_range_for_source(
                            &file.source,
                            u32::try_from(start).unwrap_or(u32::MAX),
                            u32::try_from(end).unwrap_or(u32::MAX),
                        ),
                        "newText": new_name,
                    })
                })
                .collect();
            if !edits.is_empty() {
                changes.insert(
                    lsp_file_uri_for_path(&file.path),
                    serde_json::Value::Array(edits),
                );
            }
        }
        Ok(serde_json::json!({ "changes": changes }))
    }

    fn hover_result(&self, request: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let uri = lsp_text_document_uri(request)?;
        let path = lsp_file_uri_path(uri)?;
        let position = lsp_text_document_position(request)?;
        let loaded = self.loaded_project_for_path(&path)?;
        let Some(file) = lsp_source_file_for_path(&loaded.files, &path) else {
            return Ok(serde_json::Value::Null);
        };
        let byte = lsp_position_to_byte(&file.source, position);
        if let Some(hover) = lsp_domain_field_hover_json(&file.source, byte) {
            return Ok(hover);
        }
        let Some(name) = identifier_at_byte(&file.source, byte) else {
            return Ok(serde_json::Value::Null);
        };
        let Some(node) = lsp_definition_node(&loaded.graph, name) else {
            return Ok(serde_json::Value::Null);
        };
        Ok(lsp_hover_json(node, &loaded.files))
    }

    fn signature_help_result(
        &self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let uri = lsp_text_document_uri(request)?;
        let path = lsp_file_uri_path(uri)?;
        let position = lsp_text_document_position(request)?;
        let loaded = self.loaded_project_for_path(&path)?;
        let Some(file) = lsp_source_file_for_path(&loaded.files, &path) else {
            return Ok(serde_json::Value::Null);
        };
        let byte = lsp_position_to_byte(&file.source, position);
        let Some((name, active_parameter)) = lsp_call_signature_context(&file.source, byte) else {
            return Ok(serde_json::Value::Null);
        };
        let Some(function) = lsp_function_stmt_by_name(&loaded.program, name) else {
            return Ok(serde_json::Value::Null);
        };
        Ok(lsp_signature_help_json(function, active_parameter))
    }

    fn inlay_hint_result(&self, request: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let uri = lsp_text_document_uri(request)?;
        let path = lsp_file_uri_path(uri)?;
        let requested_range = lsp_request_range(request)?;
        let loaded = self.loaded_project_for_path(&path)?;
        let Some(file) = lsp_source_file_for_path(&loaded.files, &path) else {
            return Ok(serde_json::Value::Array(Vec::new()));
        };
        let start = lsp_position_to_byte(&file.source, requested_range.0);
        let end = lsp_position_to_byte(&file.source, requested_range.1);
        Ok(serde_json::Value::Array(lsp_inlay_hints_json(
            &loaded.program,
            &file.source,
            start,
            end,
        )))
    }

    fn document_formatting_result(
        &self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let uri = lsp_text_document_uri(request)?;
        let path = lsp_file_uri_path(uri)?;
        let loaded = self.loaded_project_for_path(&path)?;
        let Some(file) = lsp_source_file_for_path(&loaded.files, &path) else {
            return Ok(serde_json::Value::Array(Vec::new()));
        };
        let formatted = lsp_format_source(
            &file.source,
            lsp_formatting_tab_size(request),
            lsp_formatting_insert_spaces(request),
        );
        if formatted == file.source {
            return Ok(serde_json::Value::Array(Vec::new()));
        }
        Ok(serde_json::Value::Array(vec![serde_json::json!({
            "range": lsp_full_document_range(&file.source),
            "newText": formatted,
        })]))
    }

    fn range_formatting_result(
        &self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let uri = lsp_text_document_uri(request)?;
        let path = lsp_file_uri_path(uri)?;
        let requested_range = lsp_request_range(request)?;
        let loaded = self.loaded_project_for_path(&path)?;
        let Some(file) = lsp_source_file_for_path(&loaded.files, &path) else {
            return Ok(serde_json::Value::Array(Vec::new()));
        };
        let (start, end, edit_range) = lsp_line_range_for_formatting(&file.source, requested_range);
        let Some(source_slice) = file.source.get(start..end) else {
            return Ok(serde_json::Value::Array(Vec::new()));
        };
        let formatted = lsp_format_source_with_initial_indent(
            source_slice,
            lsp_formatting_tab_size(request),
            lsp_formatting_insert_spaces(request),
            lsp_indent_level_before(&file.source, start),
        );
        if formatted == source_slice {
            return Ok(serde_json::Value::Array(Vec::new()));
        }
        Ok(serde_json::Value::Array(vec![serde_json::json!({
            "range": edit_range,
            "newText": formatted,
        })]))
    }

    fn on_type_formatting_result(
        &self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let trigger = request
            .pointer("/params/ch")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("onTypeFormatting ch must be a string"))?;
        if !matches!(trigger, "}" | "{" | "\n") {
            return Ok(serde_json::Value::Array(Vec::new()));
        }
        let uri = lsp_text_document_uri(request)?;
        let path = lsp_file_uri_path(uri)?;
        let position = lsp_text_document_position(request)?;
        let loaded = self.loaded_project_for_path(&path)?;
        let Some(file) = lsp_source_file_for_path(&loaded.files, &path) else {
            return Ok(serde_json::Value::Array(Vec::new()));
        };
        if trigger == "\n" {
            return Ok(lsp_newline_on_type_formatting_edit_json(
                &file.source,
                position.0,
                lsp_formatting_tab_size(request),
                lsp_formatting_insert_spaces(request),
            ));
        }
        let (start, end, edit_range) =
            lsp_current_line_range_for_formatting(&file.source, position.0);
        let Some(source_slice) = file.source.get(start..end) else {
            return Ok(serde_json::Value::Array(Vec::new()));
        };
        let formatted = lsp_format_source_with_initial_indent(
            source_slice,
            lsp_formatting_tab_size(request),
            lsp_formatting_insert_spaces(request),
            lsp_indent_level_before(&file.source, start),
        );
        if formatted == source_slice {
            return Ok(serde_json::Value::Array(Vec::new()));
        }
        Ok(serde_json::Value::Array(vec![serde_json::json!({
            "range": edit_range,
            "newText": formatted,
        })]))
    }

    fn document_symbol_result(
        &self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let uri = lsp_text_document_uri(request)?;
        let path = lsp_file_uri_path(uri)?;
        let loaded = self.loaded_project_for_path(&path)?;
        Ok(serde_json::Value::Array(
            lsp_document_symbols_protocol_json(&loaded.graph, &loaded.files),
        ))
    }

    fn code_lens_result(&self, request: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let uri = lsp_text_document_uri(request)?;
        let path = lsp_file_uri_path(uri)?;
        let loaded = self.loaded_project_for_path(&path)?;
        let Some(file) = lsp_source_file_for_path(&loaded.files, &path) else {
            return Ok(serde_json::Value::Array(Vec::new()));
        };
        Ok(serde_json::Value::Array(lsp_code_lenses_json(
            &loaded.graph,
            &loaded.files,
            file.id,
        )))
    }

    fn code_action_result(&self, request: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let uri = lsp_text_document_uri(request)?;
        let path = lsp_file_uri_path(uri)?;
        let requested_range = lsp_request_range(request)?;
        let loaded = self.loaded_project_for_path(&path)?;
        let Some(file) = lsp_source_file_for_path(&loaded.files, &path) else {
            return Ok(serde_json::Value::Array(Vec::new()));
        };
        let start = lsp_position_to_byte(&file.source, requested_range.0);
        let end = lsp_position_to_byte(&file.source, requested_range.1);
        Ok(serde_json::Value::Array(lsp_code_actions_json(
            &loaded, file, start, end,
        )))
    }

    fn document_link_result(
        &self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let uri = lsp_text_document_uri(request)?;
        let path = lsp_file_uri_path(uri)?;
        let loaded = self.loaded_project_for_path(&path)?;
        let Some(file) = lsp_source_file_for_path(&loaded.files, &path) else {
            return Ok(serde_json::Value::Array(Vec::new()));
        };
        Ok(serde_json::Value::Array(lsp_document_links_json(
            &loaded.graph,
            &loaded.files,
            file.id,
        )))
    }

    fn folding_range_result(
        &self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let uri = lsp_text_document_uri(request)?;
        let path = lsp_file_uri_path(uri)?;
        let loaded = self.loaded_project_for_path(&path)?;
        let Some(file) = lsp_source_file_for_path(&loaded.files, &path) else {
            return Ok(serde_json::Value::Array(Vec::new()));
        };
        Ok(serde_json::Value::Array(lsp_folding_ranges_json(
            &loaded.graph,
            &loaded.files,
            file.id,
        )))
    }

    fn selection_range_result(
        &self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let uri = lsp_text_document_uri(request)?;
        let path = lsp_file_uri_path(uri)?;
        let loaded = self.loaded_project_for_path(&path)?;
        let Some(file) = lsp_source_file_for_path(&loaded.files, &path) else {
            return Ok(serde_json::Value::Array(Vec::new()));
        };
        let positions = request
            .pointer("/params/positions")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| anyhow::anyhow!("positions must be an array"))?;
        let mut ranges = Vec::with_capacity(positions.len());
        for position in positions {
            let position = lsp_position_value(position)?;
            let byte = lsp_position_to_byte(&file.source, position);
            ranges.push(
                lsp_selection_range_json(&loaded.graph, &loaded.files, file.id, byte)
                    .unwrap_or_else(|| {
                        let byte = u32::try_from(byte).unwrap_or(u32::MAX);
                        serde_json::json!({
                            "range": lsp_range_for_source(&file.source, byte, byte),
                        })
                    }),
            );
        }
        Ok(serde_json::Value::Array(ranges))
    }

    fn semantic_tokens_result(
        &self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let uri = lsp_text_document_uri(request)?;
        let path = lsp_file_uri_path(uri)?;
        let loaded = self.loaded_project_for_path(&path)?;
        let Some(file) = lsp_source_file_for_path(&loaded.files, &path) else {
            return Ok(serde_json::json!({ "data": [] }));
        };
        Ok(lsp_semantic_tokens_json(
            &loaded.graph,
            &loaded.files,
            file.id,
        ))
    }

    fn completion_result(&self, request: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let uri = lsp_text_document_uri(request)?;
        let path = lsp_file_uri_path(uri)?;
        let loaded = self.loaded_project_for_path(&path)?;
        let context = if let Some(file) = lsp_source_file_for_path(&loaded.files, &path) {
            let position = lsp_text_document_position(request)?;
            let byte = lsp_position_to_byte(&file.source, position);
            lsp_completion_context(&file.source, byte)
        } else {
            LspCompletionContext::General
        };
        Ok(serde_json::json!({
            "isIncomplete": false,
            "items": lsp_completion_items_json(&loaded.graph, &loaded.files, context),
        }))
    }

    fn workspace_symbol_result(
        &self,
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let query = request
            .pointer("/params/query")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let root = self.workspace_root.as_ref().ok_or_else(|| {
            anyhow::anyhow!("initialize.params.rootUri is required before workspace/symbol")
        })?;
        let entry = project_entry_path(root)?;
        let loaded = self.loaded_project_for_path(&entry)?;
        Ok(serde_json::Value::Array(lsp_workspace_symbols_json(
            &loaded.graph,
            &loaded.files,
            query,
        )))
    }

    fn loaded_project_for_path(&self, path: &Path) -> anyhow::Result<orv_project::LoadedProject> {
        if self.open_documents.is_empty() {
            return orv_project::load_project(path).map_err(|e| anyhow::anyhow!("{e}"));
        }
        let loaded = match orv_project::load_project(path) {
            Ok(loaded) => loaded,
            Err(err) => {
                if let Some(source) = self.open_document_source_for_path(path) {
                    return orv_project::load_project_from_sources(
                        path,
                        [(path.to_path_buf(), source.clone())],
                    )
                    .map_err(|e| anyhow::anyhow!("{e}"));
                }
                return Err(anyhow::anyhow!("{err}"));
            }
        };
        let sources = loaded.files.iter().map(|file| {
            (
                file.path.clone(),
                self.open_document_source_for_path(&file.path)
                    .cloned()
                    .unwrap_or_else(|| file.source.clone()),
            )
        });
        let entry = lsp_source_file_for_path(&loaded.files, path)
            .map(|file| file.path.clone())
            .unwrap_or_else(|| path.to_path_buf());
        orv_project::load_project_from_sources(&entry, sources).map_err(|e| anyhow::anyhow!("{e}"))
    }

    fn open_document_source_for_path(&self, path: &Path) -> Option<&String> {
        if let Some(source) = self.open_documents.get(path) {
            return Some(source);
        }
        let normalized = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        self.open_documents.iter().find_map(|(open_path, source)| {
            let open_normalized = open_path
                .canonicalize()
                .unwrap_or_else(|_| open_path.clone());
            (open_path == path || open_path == &normalized || open_normalized == normalized)
                .then_some(source)
        })
    }

    fn loaded_project_for_text_document(
        &self,
        path: &Path,
    ) -> anyhow::Result<orv_project::LoadedProject> {
        if let Some(root) = &self.workspace_root {
            if let Ok(entry) = project_entry_path(root) {
                let loaded = self.loaded_project_for_path(&entry)?;
                if lsp_source_file_for_path(&loaded.files, path).is_some() {
                    return Ok(loaded);
                }
            }
        }
        self.loaded_project_for_path(path)
    }

    pub(crate) fn handle_notification(&mut self, request: &serde_json::Value) {
        match request.get("method").and_then(serde_json::Value::as_str) {
            Some("textDocument/didOpen") => self.handle_did_open(request),
            Some("textDocument/didChange") => self.handle_did_change(request),
            Some("textDocument/didSave") => self.handle_did_save(request),
            Some("textDocument/didClose") => self.handle_did_close(request),
            _ => {}
        }
    }

    fn handle_did_open(&mut self, request: &serde_json::Value) {
        let Some(uri) = request
            .pointer("/params/textDocument/uri")
            .and_then(serde_json::Value::as_str)
        else {
            return;
        };
        let Some(text) = request
            .pointer("/params/textDocument/text")
            .and_then(serde_json::Value::as_str)
        else {
            return;
        };
        let Ok(path) = lsp_file_uri_path(uri) else {
            return;
        };
        self.open_documents.insert(path, text.to_string());
    }

    fn handle_did_close(&mut self, request: &serde_json::Value) {
        let Some(uri) = request
            .pointer("/params/textDocument/uri")
            .and_then(serde_json::Value::as_str)
        else {
            return;
        };
        let Ok(path) = lsp_file_uri_path(uri) else {
            return;
        };
        self.open_documents.remove(&path);
    }

    fn handle_did_save(&mut self, request: &serde_json::Value) {
        let Some(uri) = request
            .pointer("/params/textDocument/uri")
            .and_then(serde_json::Value::as_str)
        else {
            return;
        };
        let Ok(path) = lsp_file_uri_path(uri) else {
            return;
        };
        let Some(text) = request
            .pointer("/params/text")
            .and_then(serde_json::Value::as_str)
        else {
            self.open_documents.remove(&path);
            return;
        };
        self.open_documents.insert(path, text.to_string());
    }

    fn handle_did_change(&mut self, request: &serde_json::Value) {
        let Some(uri) = request
            .pointer("/params/textDocument/uri")
            .and_then(serde_json::Value::as_str)
        else {
            return;
        };
        let Some(text) = request
            .pointer("/params/contentChanges")
            .and_then(serde_json::Value::as_array)
            .and_then(|changes| changes.last())
            .and_then(|change| change.get("text"))
            .and_then(serde_json::Value::as_str)
        else {
            return;
        };
        let Ok(path) = lsp_file_uri_path(uri) else {
            return;
        };
        self.open_documents.insert(path, text.to_string());
    }
}
