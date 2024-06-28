use nu_plugin::{serve_plugin, MsgPackSerializer, Plugin, PluginCommand};
use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::ir::IrBlock;
use nu_protocol::{
    BlockId, Category, Example, IntoSpanned, LabeledError, ShellError, Signature, Span, Type, Value,
};
use serde::Deserialize;

mod ui;

pub struct ExploreIrPlugin;

impl Plugin for ExploreIrPlugin {
    fn version(&self) -> String {
        // This automatically uses the version of your package from Cargo.toml as the plugin version
        // sent to Nushell
        env!("CARGO_PKG_VERSION").into()
    }

    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![
            // Commands should be added here
            Box::new(ExploreIr),
        ]
    }
}

#[derive(Deserialize)]
struct ViewIrOutput {
    block_id: BlockId,
    span: Span,
    ir_block: IrBlock,
}

pub struct ExploreIr;

impl SimplePluginCommand for ExploreIr {
    type Plugin = ExploreIrPlugin;

    fn name(&self) -> &str {
        "explore ir"
    }

    fn signature(&self) -> Signature {
        Signature::build(PluginCommand::name(self))
            .input_output_type(Type::String, Type::Nothing)
            .category(Category::Viewers)
    }

    fn usage(&self) -> &str {
        "Explore the output of `view ir --json` in a TUI"
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            example: "view ir --json { 1 + 2 } | explore ir",
            description: "Open a terminal viewer for the IR of the { 1 + 2 } block",
            result: None,
        }]
    }

    fn run(
        &self,
        _plugin: &ExploreIrPlugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        let json = input.as_str()?;

        let view_ir_output: ViewIrOutput = serde_json::from_str(json).map_err(|err| {
            LabeledError::new("Failed to parse output of `view ir`")
                .with_label(err.to_string(), input.span())
        })?;

        let block_contents =
            String::from_utf8_lossy(&engine.get_span_contents(view_ir_output.span)?).into_owned();

        if engine.is_using_stdio() {
            return Err(
                LabeledError::new("Plugin can't run under stdio mode").with_label(
                    "check that local socket mode is possible before running this command",
                    call.head,
                ),
            );
        }

        let foreground = engine.enter_foreground()?;
        ui::start(&view_ir_output, &block_contents)
            .map_err(|err| ShellError::from(err.into_spanned(call.head)))?;
        drop(foreground);

        Ok(Value::nothing(call.head))
    }
}

fn main() {
    serve_plugin(&ExploreIrPlugin, MsgPackSerializer);
}
