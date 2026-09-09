//! Function call state is restored before either values or errors reach callers.

use super::{Interp, LambdaValue, RuntimeError, Value};
use orv_hir::{HirFunctionBody, HirFunctionStmt, HirParam, NameId};
use std::collections::HashMap;
use std::io::Write;

impl<W: Write> Interp<W> {
    pub(super) fn call_lambda(
        &mut self,
        lambda: &LambdaValue,
        args: Vec<Value>,
    ) -> Result<Value, RuntimeError> {
        if args.len() != lambda.params.len() {
            return Err(RuntimeError::native(format!(
                "lambda expects {} arguments, got {}",
                lambda.params.len(),
                args.len()
            )));
        }
        self.with_call_scope(lambda.env.clone(), |interp| {
            interp.bind_call_params(&lambda.params, args);
            interp.eval_call_body(&lambda.body)
        })
    }

    pub(super) fn call_function(
        &mut self,
        function: &HirFunctionStmt,
        args: Vec<Value>,
    ) -> Result<Value, RuntimeError> {
        self.call_function_with_extras(function, args, Vec::new())
    }

    /// User domains pass token-slot bindings alongside their ordinary parameters.
    pub(super) fn call_function_with_extras(
        &mut self,
        function: &HirFunctionStmt,
        args: Vec<Value>,
        extras: Vec<(NameId, Value)>,
    ) -> Result<Value, RuntimeError> {
        if args.len() != function.params.len() {
            return Err(RuntimeError::native(format!(
                "function `{}` expects {} arguments, got {}",
                function.name.name,
                function.params.len(),
                args.len()
            )));
        }
        self.with_call_scope(self.env.clone(), |interp| {
            interp.bind_call_params(&function.params, args);
            interp.env.extend(extras);
            interp.debug_push_call(&function.name.name, function.span);
            let result = interp.eval_call_body(&function.body);
            interp.debug_pop_call();
            result
        })
    }

    fn bind_call_params(&mut self, params: &[HirParam], args: Vec<Value>) {
        for (parameter, value) in params.iter().zip(args) {
            self.env.insert(parameter.name.id, value);
        }
        self.debug_register_params(params);
    }

    fn eval_call_body(&mut self, body: &HirFunctionBody) -> Result<Value, RuntimeError> {
        match body {
            HirFunctionBody::Block(block) => self.eval_block_ctl(block).map(|flow| {
                self.pending_return = None;
                flow.into_value()
            }),
            HirFunctionBody::Expr(expression) => self.eval(expression),
        }
    }

    fn with_call_scope(
        &mut self,
        env: HashMap<NameId, Value>,
        call: impl FnOnce(&mut Self) -> Result<Value, RuntimeError>,
    ) -> Result<Value, RuntimeError> {
        let saved_env = std::mem::replace(&mut self.env, env);
        let saved_return = self.pending_return.take();
        let saved_html = self.html_buffer.take();
        let saved_loop = self.loop_signal;
        let result = call(self);
        self.env = saved_env;
        self.html_buffer = saved_html;
        self.pending_return = if self.response.is_some() {
            Some(Value::Void)
        } else {
            saved_return
        };
        self.loop_signal = saved_loop;
        result
    }
}
