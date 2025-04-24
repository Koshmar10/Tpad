use std::env;

use color_eyre;
use ratatui;
use tpad::*;

/*
Goals
Terminal file editor,
loading a file / multiple files
searching in file, counting words,
use std::env;


*/

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    // Initialize the terminal
    let terminal = ratatui::init();

    // Use a scope to ensure cleanup happens even if the app panics or errors
    let result = {
        let args: Vec<String> = env::args().collect();
        let file_args: Vec<String> = args.into_iter().skip(1).collect();
        let mut app = App::new(file_args);

        app.run(terminal)
    };

    // Restore the terminal to its original state
    ratatui::restore();

    // Return the result of the application
    result
}
