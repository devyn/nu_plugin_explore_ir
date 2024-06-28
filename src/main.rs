use nu_plugin::{MsgPackSerializer, Plugin, PluginCommand, serve_plugin};
use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{Category, Example, LabeledError, Signature, SyntaxShape, Value};

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
            .required("name", SyntaxShape::String, "(FIXME) A demo parameter - your name")
            .switch("shout", "(FIXME) Yell it instead", None)
            .category(Category::Experimental)
    }

    fn usage(&self) -> &str {
        "(FIXME) help text for explore ir"
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "explore ir Ellie",
                description: "Say hello to Ellie",
                result: Some(Value::test_string("Hello, Ellie. How are you today?")),
            },
            Example {
                example: "explore ir --shout Ellie",
                description: "Shout hello to Ellie",
                result: Some(Value::test_string("HELLO, ELLIE. HOW ARE YOU TODAY?")),
            },
        ]
    }

    fn run(
        &self,
        _plugin: &ExploreIrPlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: &Value,
    ) -> Result<Value, LabeledError> {
        let name: String = call.req(0)?;
        let mut greeting = format!("Hello, {name}. How are you today?");
        if call.has_flag("shout")? {
            greeting = greeting.to_uppercase();
        }
        Ok(Value::string(greeting, call.head))
    }
}

#[test]
fn test_examples() -> Result<(), nu_protocol::ShellError> {
    use nu_plugin_test_support::PluginTest;

    // This will automatically run the examples specified in your command and compare their actual
    // output against what was specified in the example. You can remove this test if the examples
    // can't be tested this way, but we recommend including it if possible.

    PluginTest::new("explore_ir", ExploreIrPlugin.into())?
        .test_command_examples(&ExploreIr)
}

fn main() {
    serve_plugin(&ExploreIrPlugin, MsgPackSerializer);
}
