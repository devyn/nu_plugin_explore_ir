use std::{
    io::{self, stdout},
    time::Duration,
};

use nu_plugin::EngineInterface;
use nu_protocol::{
    ir::{Instruction, Literal},
    Value,
};
use ratatui::{
    crossterm::{
        event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
        ExecutableCommand,
    },
    prelude::*,
    widgets::*,
};

use crate::data::{self, BlockState, ViewIrOutput};

struct State {
    engine: EngineInterface,
    head: nu_protocol::Span,
    blocks: Vec<BlockState>,
    inst_list: List<'static>,
    jump_list: Vec<JumpState>,
    should_quit: bool,
    show_inspector: bool,
    goto: bool,
    goto_contents: String,
    error: Option<String>,
}

impl State {
    fn current_block(&self) -> &BlockState {
        self.blocks.last().expect("State.blocks is empty!")
    }

    fn current_block_mut(&mut self) -> &mut BlockState {
        self.blocks.last_mut().expect("State.blocks is empty!")
    }

    fn list_state(&self) -> &ListState {
        &self.current_block().list_state
    }

    fn list_state_mut(&mut self) -> &mut ListState {
        &mut self.current_block_mut().list_state
    }
}

enum JumpState {
    IntoBlock,
    Goto { previous: usize },
}

pub(crate) fn start(
    engine: EngineInterface,
    head: nu_protocol::Span,
    initial_block: BlockState,
) -> io::Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut result = Ok(());
    let mut state = State {
        engine,
        head,
        blocks: vec![],
        inst_list: List::default(),
        jump_list: vec![],
        should_quit: false,
        show_inspector: false,
        goto: false,
        goto_contents: String::new(),
        error: None,
    };

    enter_block(&mut state, initial_block);

    state.list_state_mut().select_first();

    while !state.should_quit {
        result = result.and(terminal.draw(|frame| ui(frame, &mut state)).map(|_| ()));
        result = result.and(handle_events(&mut state));
    }

    disable_raw_mode()
        .and(stdout().execute(LeaveAlternateScreen))
        .and(result)
}

fn enter_block(state: &mut State, block: BlockState) {
    state.blocks.push(block);
    restore_block_state(state);
}

fn restore_block_state(state: &mut State) {
    if let Some(block) = state.blocks.last() {
        state.inst_list = make_instruction_list(&block.view_ir);
    } else {
        state.inst_list = List::default();
    }
}

fn go_forward(state: &mut State) {
    if let Err(err) = (|| {
        let Some(block) = state.blocks.last_mut() else {
            return Err("not in a block".into());
        };

        let Some(index) = block.list_state.selected() else {
            return Err("nothing is selected".into());
        };

        let Some(instruction) = block.view_ir.ir_block.instructions.get(index) else {
            return Err("can't find selected instruction".into());
        };

        match instruction {
            Instruction::Call { decl_id, .. } => {
                let new_block = data::get(
                    &state.engine,
                    Value::int(
                        i64::try_from(*decl_id).map_err(|err| err.to_string())?,
                        state.head,
                    ),
                    true,
                    state.head,
                )
                .map_err(|err| err.to_string())?;
                state.jump_list.push(JumpState::IntoBlock);
                enter_block(state, new_block);
                Ok(())
            }
            Instruction::LoadLiteral {
                lit:
                    Literal::Block(block_id)
                    | Literal::Closure(block_id)
                    | Literal::RowCondition(block_id),
                ..
            } => {
                // Jump into a literal block/closure/row condition
                let new_block = data::get(
                    &state.engine,
                    Value::int(
                        i64::try_from(*block_id).map_err(|err| err.to_string())?,
                        state.head,
                    ),
                    false,
                    state.head,
                )
                .map_err(|err| err.to_string())?;
                state.jump_list.push(JumpState::IntoBlock);
                enter_block(state, new_block);
                Ok(())
            }
            _ => {
                if let Some(branch_target) = instruction.branch_target() {
                    state.jump_list.push(JumpState::Goto { previous: index });
                    block.list_state.select(Some(branch_target));
                    Ok(())
                } else {
                    Err("nothing to jump to".into())
                }
            }
        }
    })() {
        state.error = Some(err);
    }
}

fn go_back(state: &mut State) {
    match state.jump_list.pop() {
        Some(JumpState::IntoBlock) => {
            if state.blocks.len() > 1 {
                state.blocks.pop();
                restore_block_state(state);
            } else {
                state.error = Some("unable to jump to the previous block".into());
            }
        }
        Some(JumpState::Goto { previous }) => {
            state.current_block_mut().list_state.select(Some(previous));
        }
        None => {
            state.error = Some("can't go back any further".into());
        }
    }
}

fn handle_events(state: &mut State) -> io::Result<()> {
    if event::poll(Duration::from_secs(1))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                handle_keypress(state, key);
            }
        }
    }
    Ok(())
}

fn handle_keypress(state: &mut State, key_event: KeyEvent) {
    state.error = None;

    match key_event.code {
        KeyCode::Char(c) if state.goto => {
            state.goto_contents.push(c);
        }
        KeyCode::Backspace if state.goto => {
            state.goto_contents.pop();
        }
        KeyCode::Enter if state.goto => {
            state.goto = false;
            match state.goto_contents.parse::<usize>() {
                Ok(index) => {
                    if index < state.inst_list.len() {
                        state.goto_contents.clear();
                        // Save so you can jump back with [
                        if let Some(previous) = state.list_state().selected() {
                            state.jump_list.push(JumpState::Goto { previous });
                        }
                        state.list_state_mut().select(Some(index));
                    } else {
                        state.error = Some("index out of range".into());
                    }
                }
                Err(err) => {
                    state.error = Some(err.to_string());
                }
            }
        }
        KeyCode::Char('q') => {
            state.should_quit = true;
        }
        KeyCode::Char('g') => {
            state.goto = true;
        }
        KeyCode::Char(' ') => {
            state.show_inspector = true;
        }
        KeyCode::Esc => {
            state.show_inspector = false;
            state.goto = false;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            state.list_state_mut().select_previous();
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.list_state_mut().select_next();
        }
        KeyCode::Char('[') => {
            go_back(state);
        }
        KeyCode::Char(']') => {
            go_forward(state);
        }
        _ => (),
    }
}

fn instruction_style(instruction: &Instruction) -> Style {
    match instruction {
        Instruction::Call { .. } => Style::new().light_cyan(),
        Instruction::LoadLiteral {
            lit: Literal::Block(_) | Literal::Closure(_) | Literal::RowCondition(_),
            ..
        } => Style::new().light_cyan(),
        _ if instruction.branch_target().is_some() => Style::new().light_green(),
        _ => Style::new(),
    }
}

fn make_instruction_list(view_ir_output: &ViewIrOutput) -> List<'static> {
    view_ir_output
        .formatted_instructions
        .iter()
        .enumerate()
        .map(|(index, inst)| {
            let instruction = &view_ir_output.ir_block.instructions[index];
            let comment = &view_ir_output.ir_block.comments[index];
            // Parse the formatted instruction into its two components so we can color it
            let (inst_name, inst_args) = if let Some(split_offset) = inst.find(' ') {
                let (inst_name, inst_args) = inst.split_at(split_offset);
                (
                    inst_name,
                    inst_args
                        .split_at(inst_args.find(|ch| ch != ' ').unwrap_or(0))
                        .1,
                )
            } else {
                (inst.as_str(), "")
            };
            Line::from_iter([
                Span::styled(format!("{index:4}: "), Style::new().dim()),
                Span::raw(format!("{inst_name:22} ")),
                // Make it stand out if it's jumpable
                Span::styled(format!("{inst_args:17}"), instruction_style(instruction)),
                if !comment.is_empty() {
                    Span::styled(format!(" # {comment}"), Style::new().dim().italic())
                } else {
                    Span::raw("")
                },
            ])
        })
        .collect()
}

fn ui(frame: &mut Frame, state: &mut State) {
    let main_layout = Layout::new(
        Direction::Vertical,
        [Constraint::Fill(1), Constraint::Max(1)],
    )
    .split(frame.size());

    // Bottom status
    statusbar_ui(frame, state, main_layout[1]);

    let layout = Layout::new(
        Direction::Horizontal,
        [Constraint::Fill(1), Constraint::Fill(1)],
    )
    .split(main_layout[0]);

    instructions_ui(frame, state, layout[0]);
    source_code_ui(frame, state, layout[1]);

    if state.show_inspector {
        inspector_ui(frame, state);
    }
}

fn statusbar_ui(frame: &mut Frame, state: &mut State, area: Rect) {
    let key_style = Style::new().light_blue().bold();
    let desc_style = Style::new().italic();

    if let Some(error) = &state.error {
        frame.render_widget(
            Line::from_iter([
                Span::styled("Error: ", Style::new().red().bold()),
                Span::styled(error, Style::new().red()),
            ]),
            area,
        );
    } else if state.goto {
        let widget = Line::from_iter([
            Span::styled("Go to index: ", desc_style),
            Span::raw(state.goto_contents.as_str()),
        ]);
        frame.set_cursor(
            u16::try_from(widget.width())
                .map(|width| (area.x + width).min(area.right()))
                .unwrap_or(area.right()),
            area.y,
        );
        frame.render_widget(widget, area);
    } else {
        frame.render_widget(
            Line::from_iter([
                Span::styled("<q>", key_style),
                Span::styled(" quit  ", desc_style),
                Span::styled("<space>", key_style),
                Span::styled(" inspect  ", desc_style),
                Span::styled("<g>", key_style),
                Span::styled(" goto  ", desc_style),
                Span::styled("<↑/↓/k/j>", key_style),
                Span::styled(" navigate  ", desc_style),
                Span::styled("<[/]>", key_style),
                Span::styled(" jump back/fwd  ", desc_style),
            ]),
            area,
        );
    }
}

fn instructions_ui(frame: &mut Frame, state: &mut State, area: Rect) {
    frame.render_stateful_widget(
        state
            .inst_list
            .clone()
            .block(Block::bordered().title(Span::styled("IR instructions", Style::new().bold())))
            .highlight_style(Style::new().reversed()),
        area,
        state.list_state_mut(),
    );
}

fn source_code_ui(frame: &mut Frame, state: &mut State, area: Rect) {
    let Some(block) = state.blocks.last() else {
        return;
    };

    // Highlight the span of the selected instruction
    let block_span = block.view_ir.span;
    let highlighted_span = block
        .list_state
        .selected()
        .and_then(|index| block.view_ir.ir_block.spans.get(index).cloned())
        .unwrap_or(nu_protocol::Span::unknown());

    let source_code_title = Span::styled("Source code", Style::new().bold());

    let mut text = Text::default();

    if highlighted_span.start >= block_span.start && highlighted_span.end <= block_span.end {
        let start = highlighted_span.start - block_span.start;
        let end = highlighted_span.end - block_span.start;
        let (initial, next) = block.source.split_at(start);
        let (highlighted, final_part) = next.split_at(end - start);

        // First, push the initial lines that have no style
        let initial_line_count = initial.lines().count();
        text.extend(
            initial
                .lines()
                .take(initial_line_count.saturating_sub(1))
                .map(Line::raw),
        );

        // The unstyled part of the last initial line
        if let Some(unstyled_part_of_last_initial_line) = initial.lines().last() {
            text.push_line(unstyled_part_of_last_initial_line);
        }

        // Now, the highlighted part
        let style = Style::new().light_blue().reversed().bold();
        let styled_line_count = highlighted.lines().count();
        let mut lines = highlighted.lines();
        if let Some(highlighted_part_of_last_initial_line) = lines.next() {
            text.push_span(Span::styled(highlighted_part_of_last_initial_line, style));
        }
        text.extend(
            (&mut lines)
                .take(styled_line_count.saturating_sub(2))
                .map(|line| Line::styled(line, style)),
        );
        if let Some(highlighted_part_of_first_final_line) = lines.next() {
            text.push_line(Span::styled(highlighted_part_of_first_final_line, style));
        }

        // The unstyled part of the first final line
        if let Some(unstyled_part_of_first_final_line) = final_part.lines().next() {
            text.push_span(unstyled_part_of_first_final_line);
        }

        // Finally, push the final lines that have no style.
        text.extend(final_part.lines().skip(1).map(Line::raw));
    } else {
        text = Text::raw(&block.source);
    }

    frame.render_widget(
        Paragraph::new(text).block(Block::bordered().title(source_code_title)),
        area,
    );
}

fn inspector_ui(frame: &mut Frame, state: &mut State) {
    // Place the dialog in the center
    let v_layout = Layout::new(
        Direction::Vertical,
        [
            Constraint::Fill(1),
            Constraint::Max(20),
            Constraint::Fill(1),
        ],
    )
    .split(frame.size());
    let h_layout = Layout::new(
        Direction::Horizontal,
        [
            Constraint::Fill(1),
            Constraint::Max(60),
            Constraint::Fill(1),
        ],
    )
    .split(v_layout[1]);
    let dialog_size = h_layout[1];

    let block = Block::bordered().title(Span::styled("Inspect instruction", Style::new().bold()));
    let block_inner = block.inner(dialog_size);
    frame.render_widget(Clear, dialog_size);
    frame.render_widget(block, dialog_size);

    let block_layout = Layout::new(
        Direction::Vertical,
        [Constraint::Max(2), Constraint::Fill(1), Constraint::Max(2)],
    )
    .split(block_inner);

    if let Some(index) = state.list_state().selected() {
        let block = state.current_block();
        let formatted_instruction = &block.view_ir.formatted_instructions[index];
        let instruction = &block.view_ir.ir_block.instructions[index];
        let debug_instruction = format!("{:#?}", instruction);

        frame.render_widget(
            Paragraph::new(Line::from_iter([
                Span::styled(format!("{index:4}: "), Style::new().dim()),
                Span::raw(formatted_instruction.as_str()),
            ]))
            .block(Block::new().borders(Borders::BOTTOM)),
            block_layout[0],
        );

        frame.render_widget(Paragraph::new(debug_instruction.as_str()), block_layout[1]);

        frame.render_widget(
            Paragraph::new(Line::from_iter([
                Span::styled("<esc>", Style::new().light_blue().bold()),
                Span::styled(" close inspector", Style::new().italic()),
            ]))
            .block(Block::new().borders(Borders::TOP)),
            block_layout[2],
        );
    }
}
