# nu_plugin_explore_ir

> [!IMPORTANT]
> This is a work in progress. You need [my IR branch](https://github.com/nushell/nushell/tree/ir) of Nushell for this to work.  
> The IR is under development and will likely change substantially before it's merged.

This is a fancy viewer for `view ir --json`. Example:

```nushell
view ir --json {
  if ($env.HELLO | is-not-empty) {
    "Hello, " ++ $env.HELLO ++ "!"
  } else {
    "Goodbye, " ++ (random uuid) ++ "!"
  }
} | explore ir
```

![An example of what the UI looks like for the above code](doc/example.png)

Key bindings:

| Key            | Effect                                                          |
| -------------- | --------------------------------------------------------------- |
| **q**          | Quit the application.                                           |
| **g**          | Go to a specific instruction by index.                          |
| **SPACE**      | Open the inspector, which shows debug info for the instruction. |
| **↑** or **k** | Go to the previous instruction.                                 |
| **↓** or **j** | Go to the next instruction.                                     |
| **ESC**        | Close a dialog box or prompt.                                   |
