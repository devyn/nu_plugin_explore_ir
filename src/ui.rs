use std::{
    io::{self, stdout},
    iter::once,
    time::Duration,
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

use crate::ViewIrOutput;

struct State<'a> {
    view_ir_output: &'a ViewIrOutput,
    block_contents: &'a str,
    inst_list_state: ListState,
    inst_list: List<'a>,
    should_quit: bool,
    show_inspector: bool,
    goto: bool,
    goto_contents: String,
    error: Option<String>,
}

pub(crate) fn start(view_ir_output: &ViewIrOutput, block_contents: &str) -> io::Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut result = Ok(());
    let mut state = State {
        view_ir_output,
        block_contents,
        inst_list_state: ListState::default(),
        inst_list: make_instruction_list(view_ir_output),
        should_quit: false,
        show_inspector: false,
        goto: false,
        goto_contents: String::new(),
        error: None,
    };

    state.inst_list_state.select_first();

    while !state.should_quit {
        result = result.and(terminal.draw(|frame| ui(frame, &mut state)).map(|_| ()));
        result = result.and(handle_events(&mut state));
    }

    disable_raw_mode()
        .and(stdout().execute(LeaveAlternateScreen))
        .and(result)
}

fn handle_events(state: &mut State) -> io::Result<()> {
    if event::poll(Duration::from_millis(50))? {
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
                        state.inst_list_state.select(Some(index));
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
            state.inst_list_state.select_previous();
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.inst_list_state.select_next();
        }
        _ => (),
    }
}

fn make_instruction_list(view_ir_output: &ViewIrOutput) -> List {
    view_ir_output
        .formatted_instructions
        .iter()
        .enumerate()
        .map(|(index, inst)| {
            Line::from_iter([
                Span::styled(format!("{index:4}: "), Style::new().dim()),
                Span::raw(inst),
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

    // Instruction list
    frame.render_stateful_widget(
        state
            .inst_list
            .clone()
            .block(Block::bordered().title(Span::styled("IR instructions", Style::new().bold())))
            .highlight_style(Style::new().reversed()),
        layout[0],
        &mut state.inst_list_state,
    );

    // Highlight the span of the selected instruction
    let block_span = state.view_ir_output.span;
    let highlighted_span = state
        .inst_list_state
        .selected()
        .and_then(|index| state.view_ir_output.ir_block.spans.get(index).cloned())
        .unwrap_or(nu_protocol::Span::unknown());

    let source_code_title = Span::styled("Source code", Style::new().bold());

    if highlighted_span.start >= block_span.start && highlighted_span.end <= block_span.end {
        let start = highlighted_span.start - block_span.start;
        let end = highlighted_span.end - block_span.start;
        let (initial, next) = state.block_contents.split_at(start);
        let (highlighted, final_part) = next.split_at(end - start);

        // First, the initial and final lines with no highlight
        let initial_line_count = initial.split("\n").count();
        let unstyled_initial_lines = initial
            .split("\n")
            .take(initial_line_count.saturating_sub(1))
            .map(Line::raw);
        let unstyled_final_lines = final_part.split("\n").skip(1).map(Line::raw);

        // The unstyled part of the last initial line and the first final line
        let unstyled_part_of_last_initial_line = initial.split("\n").last().map(Span::raw);
        let unstyled_part_of_first_final_line = final_part.split("\n").next().map(Span::raw);

        // Now, the highlighted part
        let style = Style::new().blue().reversed().bold();
        let styled_part_of_initial_line = highlighted
            .split("\n")
            .next()
            .map(|s| Span::styled(s, style));
        let styled_part_of_final_line = highlighted
            .split("\n")
            .skip(1)
            .last()
            .map(|s| Span::styled(s, style));
        let styled_line_count = highlighted.split("\n").count();
        let styled_middle_lines = highlighted
            .split("\n")
            .skip(1)
            .take(styled_line_count.saturating_sub(2))
            .map(|s| Line::styled(s, style));

        // Put it all together:
        let lines = if styled_line_count == 1 {
            Text::from_iter(
                unstyled_initial_lines
                    .chain(once(Line::from_iter(
                        [
                            unstyled_part_of_last_initial_line,
                            styled_part_of_initial_line,
                            styled_part_of_final_line,
                            unstyled_part_of_first_final_line,
                        ]
                        .into_iter()
                        .flatten(),
                    )))
                    .chain(unstyled_final_lines),
            )
        } else {
            Text::from_iter(
                unstyled_initial_lines
                    .chain(once(Line::from_iter(
                        [
                            unstyled_part_of_last_initial_line,
                            styled_part_of_initial_line,
                        ]
                        .into_iter()
                        .flatten(),
                    )))
                    .chain(styled_middle_lines)
                    .chain(once(Line::from_iter(
                        [styled_part_of_final_line, unstyled_part_of_first_final_line]
                            .into_iter()
                            .flatten(),
                    )))
                    .chain(unstyled_final_lines),
            )
        };
        frame.render_widget(
            Paragraph::new(lines).block(Block::bordered().title(source_code_title)),
            layout[1],
        );
    } else {
        frame.render_widget(
            Paragraph::new(state.block_contents).block(Block::bordered().title(source_code_title)),
            layout[1],
        );
    }

    if state.show_inspector {
        inspector_ui(frame, state);
    }
}

fn statusbar_ui(frame: &mut Frame, state: &mut State, area: Rect) {
    let key_style = Style::new().blue().bold();
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
                Span::styled("<up/k>", key_style),
                Span::styled(" previous  ", desc_style),
                Span::styled("<down/j>", key_style),
                Span::styled(" next  ", desc_style),
            ]),
            area,
        );
    }
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

    if let Some(index) = state.inst_list_state.selected() {
        let formatted_instruction = &state.view_ir_output.formatted_instructions[index];
        let instruction = &state.view_ir_output.ir_block.instructions[index];
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
                Span::styled("<esc>", Style::new().blue().bold()),
                Span::styled(" close inspector", Style::new().italic()),
            ]))
            .block(Block::new().borders(Borders::TOP)),
            block_layout[2],
        );
    }
}
