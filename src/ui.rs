use std::{
    io::{self, stdout},
    time::Duration,
};

use ratatui::{
    crossterm::{
        event::{self, Event, KeyCode, KeyEventKind},
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
        should_quit: false,
    };
    while !state.should_quit {
        result = result.and(terminal.draw(|frame| ui(frame, &state)).map(|_| ()));
        result = result.and(handle_events(&mut state));
    }

    disable_raw_mode()
        .and(stdout().execute(LeaveAlternateScreen))
        .and(result)
}

fn handle_events(state: &mut State) -> io::Result<()> {
    if event::poll(Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                state.should_quit = true;
            }
        }
    }
    Ok(())
}

fn ui(frame: &mut Frame, state: &State) {
    frame.render_widget(Paragraph::new(state.block_contents), frame.size());
}
