# tpad - A Terminal Text Editor

`tpad` is a terminal-based text editor built in Rust, utilizing the `ratatui` library for its user interface. It aims to provide essential editing features in a lightweight and customizable environment.

## Core Features

*   **Multi-File Editing:** Open and edit multiple files simultaneously.
*   **Tabbed Interface:** Each open file is displayed in its own tab for easy navigation.
*   **Basic Text Editing:**
    *   Character insertion and deletion.
    *   Line splitting (Enter) and merging (Backspace at the start of a line).
*   **Selection:** Select text using `Shift + Arrow Keys`.
*   **Clipboard:** Copy (`Ctrl+C`) and Paste (`Ctrl+V`) functionality.
*   **Undo/Redo:** Unlimited undo (`Ctrl+Z`) and redo (`Ctrl+Y`) for text operations.
*   **Command Mode:** Activated by pressing `:`, allowing for various operations:
    *   **File Operations:** Open (`o <path>`), save (`w`), save and quit (`wq`), close tab (`q`), exit editor (`cl`).
    *   **Search:** Find text within the current document (`/<term>`). Navigate matches with `Alt+N` (next) and `Alt+M` (previous).
    *   **Word Count:** Count occurrences of a specific word (`count <word>`).
    *   **Theme Management:** Open the theme configuration file (`theme`), select a theme (`set`).
*   **Session Persistence:** `tpad` saves your open files and their undo/redo history, restoring them when you next start the application.
*   **Customizable Theming:** Modify the editor's appearance by editing the `theme.toml` file.

## Keybindings

### General
*   `Ctrl+Q`: Quit application (prompts if any file has unsaved changes).

### Mode Switching
*   `:`: Enter Command mode from Editor mode.
*   `Esc`: Enter Editor mode from Command mode or close popups.

### Editor Mode
*   **Navigation:**
    *   `Arrow Keys`: Move cursor.
    *   `Shift + Arrow Keys`: Extend selection.
*   **Editing:**
    *   `Enter`: Split line / Insert new line.
    *   `Backspace`: Delete character to the left of the cursor. If at the beginning of a line (and not the first line), merges with the previous line.
    *   `Ctrl+C`: Copy selected text to the clipboard.
    *   `Ctrl+V`: Paste text from the clipboard.
    *   `Ctrl+Z`: Undo last operation.
    *   `Ctrl+Y`: Redo last undone operation.
    *   `Ctrl+S`: Save the current active file.
*   **Tab Management:**
    *   `Alt+Left Arrow`: Switch to the previous tab.
    *   `Alt+Right Arrow`: Switch to the next tab.
    *   `Alt+<number>` (e.g., `Alt+1`): Switch to the specified tab number.

### Command Mode
(Enter the command then press `Enter` to execute)
*   `o <filepath>` or `o <file1> <file2> ...`: Open one or more files.
*   `w`: Save the current file.
*   `wq`: Save the current file and close its tab.
*   `q`: Close the current tab (prompts if there are unsaved changes).
*   `cl`: Exit `tpad` (prompts if any file has unsaved changes).
*   `/<search_term>`: Find `search_term` in the current document. Focus shifts to Editor mode.
    *   In Editor mode after a search:
        *   `Alt+N`: Go to the next search match.
        *   `Alt+M`: Go to the previous search match.
*   `count <word>`: Display a count of `word` in the current document (output to console).
*   `list`: List all open documents (output to console).
*   `theme`: Open the `theme.toml` configuration file in a new tab.
*   `set`: Open a popup to select from available themes.
*   `clundo`: Clear the undo/redo history for the current document.

## Building from Source

1.  **Prerequisites:**
    *   Ensure you have the Rust toolchain (Rustc, Cargo) installed. Visit [rust-lang.org](https://www.rust-lang.org/tools/install) for installation instructions.
2.  **Clone the Repository:**
    ```bash
    git clone <your_repository_url>
    cd tpad
    ```
3.  **Build:**
    *   For a debug build:
        ```bash
        cargo build
        ```
    *   For a release build (recommended for performance):
        ```bash
        cargo build --release
        ```
4.  **Run:**
    *   From the project root:
        ```bash
        ./target/debug/tpad [file1 file2 ...]
        ```
        or for a release build:
        ```bash
        ./target/release/tpad [file1 file2 ...]
        ```

## Configuration

### Theming
`tpad` supports custom themes via a TOML file.
*   The theme file is typically located at `~/.config/tpad/theme.toml` (this path might vary slightly based on your OS conventions for configuration directories).
*   If the theme file or directory does not exist, `tpad` will attempt to create a default one when it first loads or when you try to save a theme.
*   You can edit this file to change colors for various UI elements like the editor background, foreground, highlights, status bar, tabs, etc.
*   Use the `:theme` command to open your current theme file directly in `tpad`.
*   Use the `:set` command to choose from available theme files in your themes directory (`~/.config/tpad/themes/`).

## Project Status & Potential Future Enhancements

`tpad` is an actively developed project. Here are some areas for future improvement:

*   **Refactor Core Logic:** Improve the structure of `App::handle_key_event` for better maintainability.
*   **Enhanced Error Handling:** Reduce reliance on `unwrap()` and provide user-friendly error messages.
*   **Comprehensive Testing:** Expand the test suite, particularly for complex editing operations.
*   **Advanced Search/Replace:** Implement features like regex search, case sensitivity options, and text replacement.
*   **Syntax Highlighting:** Add support for syntax highlighting for various programming languages.
*   **Auto-Indentation:** Implement intelligent auto-indentation.
*   **New File Creation:** Allow creating new, untitled files from within the editor.
*   **Visual Line Wrapping:** Add an option for visual line wrapping.
*   **Improved Navigation:** Introduce more sophisticated cursor movement commands (e.g., by word, to start/end of paragraph).
*   **In-App Command Output:** Display output from commands like `:list` within the TUI.

Contributions and suggestions are welcome!