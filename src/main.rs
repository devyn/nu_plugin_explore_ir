use nu_plugin::{serve_plugin, MsgPackSerializer, Plugin, PluginCommand};
use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{
    Category, Example, IntoSpanned, LabeledError, ShellError, Signature, SyntaxShape, Value,
};

mod data;
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

pub struct ExploreIr;

impl SimplePluginCommand for ExploreIr {
    type Plugin = ExploreIrPlugin;

    fn name(&self) -> &str {
        "explore ir"
    }

    fn signature(&self) -> Signature {
        Signature::build(PluginCommand::name(self))
            .required(
                "target",
                SyntaxShape::Any,
                "The name or block to explore compiled code for.",
            )
            .switch(
                "decl-id",
                "Integer is a declaration ID rather than a block ID.",
                Some('d'),
            )
            .category(Category::Viewers)
    }

    fn usage(&self) -> &str {
        "Explore the IR of a block or definition."
    }

    fn extra_usage(&self) -> &str {
        "Accepts valid arguments for `view ir`. For more information, see `view ir --help`."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "explore ir { 1 + 2 }",
                description: "Open a terminal viewer for the IR of the { 1 + 2 } block.",
                result: None,
            },
            Example {
                example: "explore ir 'std bench'",
                description: "Explore IR for the 'std bench' command. Only works for custom commands (written in Nushell).",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        _plugin: &ExploreIrPlugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: &Value,
    ) -> Result<Value, LabeledError> {
        let target = call.req(0)?;
        let is_decl_id = call.has_flag("decl-id")?;

        let initial_block = data::get(engine, target, is_decl_id, call.head)?;

        if engine.is_using_stdio() {
            return Err(
                LabeledError::new("Plugin can't run under stdio mode").with_label(
                    "check that local socket mode is possible before running this command",
                    call.head,
                ),
            );
        }

        let foreground = engine.enter_foreground()?;
        ui::start(engine.clone(), call.head, initial_block)
            .map_err(|err| ShellError::from(err.into_spanned(call.head)))?;
        drop(foreground);

        Ok(Value::nothing(call.head))
    }
}

fn main() {
    serve_plugin(&ExploreIrPlugin, MsgPackSerializer);
}
