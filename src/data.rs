use nu_plugin::{EngineInterface, EvaluatedCall};
use nu_protocol::{ir::IrBlock, BlockId, IntoSpanned, LabeledError, PipelineData, Span, Value};
use ratatui::widgets::ListState;
use serde::Deserialize;

#[derive(Deserialize)]
#[allow(unused)]
pub struct ViewIrOutput {
    pub block_id: BlockId,
    pub span: Span,
    pub ir_block: IrBlock,
    pub formatted_instructions: Vec<String>,
}

fn view_ir(
    engine: &EngineInterface,
    target: Value,
    is_decl_id: bool,
    head: Span,
) -> Result<ViewIrOutput, LabeledError> {
    let Some(decl_id) = engine.find_decl("view ir")? else {
        return Err(LabeledError::new("Can't find `view ir`")
            .with_label("must be in scope for `explore ir`", head));
    };

    let target_span = target.span();

    let result = engine.call_decl(
        decl_id,
        EvaluatedCall::new(head)
            .with_named("json".into_spanned(head), Value::bool(true, head))
            .with_named("decl-id".into_spanned(head), Value::bool(is_decl_id, head))
            .with_positional(target),
        PipelineData::Empty,
        true,
        false,
    )?;

    let json = result.into_value(head)?.into_string()?;

    serde_json::from_str(&json).map_err(|err| {
        LabeledError::new("Failed to parse output of `view ir`")
            .with_label(err.to_string(), target_span)
    })
}

pub struct BlockState {
    pub view_ir: ViewIrOutput,
    pub source: String,
    pub list_state: ListState,
}

pub fn get(
    engine: &EngineInterface,
    target: Value,
    is_decl_id: bool,
    head: Span,
) -> Result<BlockState, LabeledError> {
    let view_ir = view_ir(engine, target, is_decl_id, head)?;

    let source = String::from_utf8_lossy(&engine.get_span_contents(view_ir.span)?).into_owned();

    Ok(BlockState {
        view_ir,
        source,
        list_state: ListState::default(),
    })
}
