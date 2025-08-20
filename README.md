# tpad - A Terminal Text Editor

`tpad` is a terminal-based text editor built in Rust, utilizing the `ratatui` library for its user interface. It aims to provide essential editing features in a lightweight and customizable environment.

## Core Features

*   Multi-File Editing: Open and edit multiple files simultaneously.
*   Tabbed Interface: Each open file is displayed in its own tab for easy navigation.
*   Basic Text Editing:
    *   Character insertion and deletion.
    *   Line splitting (Enter) and merging (Backspace at the start of a line).
*   Selection: Select text using `Shift + Arrow Keys`.
*   Clipboard: Copy (`Ctrl+C`) and Paste (`Ctrl+V`) functionality.
*   Undo/Redo: Unlimited undo (`Ctrl+Z`) and redo (`Ctrl+Y`) for text operations.
*   Command Mode: Activated by pressing `:`, allowing for various operations:
    *   File Operations: Open (`o <path>`), save (`w`), save and quit (`wq`), close tab (`q`), exit editor (`cl`).
    *   Search: Find text within the current document (`/<term>`). Navigate matches with `Alt+N` (next) and `Alt+M` (previous).
    *   Word Count: Count occurrences of a specific word (`count <word>`).
    *   Theme Management: Open the theme configuration file (`theme`), select a theme (`set`).
*   Default Directory & Path Resolution:
    *   tpad keeps a “default directory” used when opening bare filenames (no path separators).
    *   `o <file>` with a bare name saves/opens under the default directory.
    *   `~/...` is expanded to your HOME. Relative/absolute paths are respected as provided.
    *   Change it with `setdir <path>` (see Command Mode).
*   Empty-State Help Panel: When no files are open, tpad shows a built-in help screen with common commands and the current default directory.
*   Status Bar Info: Shows line/column, permissions, Saved/Unsaved, size, tab count, and the current default directory.
*   Session Persistence: tpad saves your open files and their undo/redo history, restoring them on next start.
*   Customizable Theming: Modify the editor's appearance by editing the `theme.toml` file.

## Keybindings

### General
*   `Ctrl+Q`: Quit application (prompts if any file has unsaved changes).

### Mode Switching
*   `:`: Enter Command mode from Editor mode.
*   `Esc`: Enter Editor mode from Command mode or close popups.

### Editor Mode
*   Navigation:
    *   `Arrow Keys`: Move cursor.
    *   `Shift + Arrow Keys`: Extend selection.
*   Editing:
    *   `Enter`: Split line / Insert new line.
    *   `Backspace`: Delete character to the left of the cursor. If at the beginning of a line (and not the first line), merges with the previous line.
    *   `Ctrl+C`: Copy selected text to the clipboard.
    *   `Ctrl+V`: Paste text from the clipboard.
    *   `Ctrl+Z`: Undo last operation.
    *   `Ctrl+Y`: Redo last undone operation.
    *   `Ctrl+S`: Save the current active file.
*   Tab Management:
    *   `Alt+Left Arrow`: Switch to the previous tab.
    *   `Alt+Right Arrow`: Switch to the next tab.
    *   `Alt+<number>` (e.g., `Alt+1`): Switch to the specified tab number.

### Command Mode
(Enter the command then press `Enter` to execute)
*   `o <path>` or `o <file1> <file2> ...`: Open one or more files.
    *   Bare names (no `/`) are placed under the default directory.
    *   `~/...` expands to HOME; absolute and relative paths are respected.
*   `setdir <path>`: Set the default directory for bare filenames (e.g., `setdir ~/Documents/notes`).
*   `w`: Save the current file.
*   `wq`: Save the current file and close its tab.
*   `q`: Close the current tab (prompts if there are unsaved changes).
*   `cl`: Exit tpad (prompts if any file has unsaved changes).
*   `/<search_term>`: Find `search_term` in the current document. Focus shifts to Editor mode.
    *   After a search in Editor mode:
        *   `Alt+N`: Next match.
        *   `Alt+M`: Previous match.
*   `count <word>`: Show count for `word` in the current document (in-app popup).
*   `list`: Show command reference and the current default directory (in-app popup).
*   `theme`: Open the `theme.toml` configuration file in a new tab.
*   `set`: Open a popup to select from available themes.
*   `clundo`: Clear the undo/redo history for the current document.

## UI Notes

*   Empty App Panel: When all tabs are closed, a help panel is shown with common commands and the current default directory. Press `:` to enter Command mode and open files.
*   Status Bar: Displays Line/Col, permissions, Saved/Unsaved, file size, tab count, and `dir: <default-dir>` (with `~` shorthand when applicable).

## Building from Source

1.  Prerequisites:
    *   Ensure you have the Rust toolchain (Rustc, Cargo) installed. Visit [rust-lang.org](https://www.rust-lang.org/tools/install) for installation instructions.
2.  Clone the Repository:
    ```bash
    git clone <your_repository_url>
    cd tpad
    ```
3.  Build:
    *   Debug:
        ```bash
        cargo build
        ```
    *   Release:
        ```bash
        cargo build --release
        ```
4.  Run:
    *   From the project root:
        ```bash
        ./target/debug/tpad [file1 file2 ...]
        ```
        or:
        ```bash
        ./target/release/tpad [file1 file2 ...]
        ```

## Configuration

### Theming
tpad supports custom themes via a TOML file.
*   The theme file is typically located at `~/.config/tpad/theme.toml` (this path might vary by OS).
*   If missing, tpad attempts to create a default file when first loading/saving a theme.
*   Edit colors for UI elements like editor background/foreground, highlights, status bar, tabs, etc.
*   Use `:theme` to open your current theme file directly in tpad.
*   Use `:set` to choose from available theme files in `~/.config/tpad/themes/`.

### Default Directory
*   Default directory is used when opening bare filenames (`o notes.txt`).
*   Initial value: `$HOME/Documents` if it exists; otherwise the current working directory.
*   Change it for the current session with:
    ```
    :setdir <path>
    ```
    Examples: `:setdir ~/Notes`, `:setdir /tmp`, `:setdir projects/tpad`
*   Paths:
    *   Bare: placed under default directory.
    *   `~/...`: expanded to HOME.
    *   Relative: used as-is relative to the current working directory.
    *   Absolute: used as-is.

## Project Status & Potential Future Enhancements

tpad is an actively developed project. Potential improvements:
*   Refactor core input handling for maintainability.
*   Enhanced error handling (eliminate `unwrap()`).
*   Comprehensive tests for editing/undo/redo/selection edge cases.
*   Advanced search/replace (regex, case sensitivity, whole-word, replace).
*   Syntax highlighting.
*   Auto-indentation.
*   New file creation workflow.
*   Visual line wrapping.
*   Richer navigation (word-wise, paragraph, go-to-line).
*   More in-app outputs for commands.

Contributions and suggestions are welcome!