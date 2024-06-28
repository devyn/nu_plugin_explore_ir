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
    should_quit: bool,
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
        should_quit: false,
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
    match key_event.code {
        KeyCode::Char('q') => {
            state.should_quit = true;
        }
        KeyCode::Up => {
            state.inst_list_state.select_previous();
        }
        KeyCode::Down => {
            state.inst_list_state.select_next();
        }
        _ => (),
    }
}

fn ui(frame: &mut Frame, state: &mut State) {
    let layout = Layout::new(
        Direction::Horizontal,
        [Constraint::Fill(1), Constraint::Fill(1)],
    )
    .split(frame.size());

    // Instruction list
    frame.render_stateful_widget(
        state
            .view_ir_output
            .formatted_instructions
            .iter()
            .enumerate()
            .map(|(index, inst)| {
                Line::from_iter([
                    Span::styled(format!("{index:4}: "), Style::new().dim()),
                    Span::raw(inst),
                ])
            })
            .collect::<List>()
            .block(Block::bordered().title("IR instructions"))
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
            Paragraph::new(lines).block(Block::bordered().title("Source code")),
            layout[1],
        );
    } else {
        frame.render_widget(
            Paragraph::new(state.block_contents).block(Block::bordered().title("Source code")),
            layout[1],
        );
    }
}
