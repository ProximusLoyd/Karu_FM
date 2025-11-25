# Karu - A Terminal File Manager

Karu is a fast, lightweight, and feature-rich terminal file manager written in Rust. It provides a TUI (Terminal User Interface) for navigating the file system, performing common file operations, and previewing files directly in your terminal.

![Screenshot](https://raw.githubusercontent.com/eunvr/karu/main/screenshot.png)

## Features

*   **Cross-Platform:** Works on Linux, macOS, and Windows.
*   **Vim-like Keybindings:** Navigate with `j`, `k`, `h`, `l`.
*   **File Operations:** Cut, copy, paste, delete (to trash), rename, create files and directories.
*   **Image Previews:** Preview images directly in the terminal (requires a compatible terminal).
*   **Text File Previews:** Preview text files.
*   **Hidden Files:** Toggle visibility of hidden files.
*   **Fuzzy Filtering:** Filter files in the current directory.
*   **Trash Support:** Files are moved to the system's trash bin by default.

## Installation

### Prerequisites

*   [Rust](https://www.rust-lang.org/tools/install)
*   A Nerd Font installed and enabled in your terminal for icons to display correctly.

### From Crates.io (Recommended)

```bash
cargo install karu
```

### From Source

1.  Clone the repository:
    ```bash
    git clone https://github.com/eunvr/karu.git
    cd karu
    ```
2.  Build it up:
    ```bash
    cargo install --release .
    ```

3.  Install it 

    Run the application locally in by installing ".local/bin" f:
    ```cp /target/release/karu .local/bin/```
    ```karu```



### OS-Specific Dependencies

#### Linux

For image previews using the sixel graphics format, you may need to install `libsixel-dev`:

*   **Debian/Ubuntu:**
    ```bash
    sudo apt-get install libsixel-dev
    ```
*   **Fedora:**
    ```bash
    sudo dnf install libsixel-devel
    ```
*   **Arch Linux:**
    ```bash
    sudo pacman -S libsixel
    ```

#### Windows and macOS

Currently under developement, for support kindly let us get funding by offering a coffee "buy me a coffee"

**Note on Image Previews in Terminals:**

While the application works out-of-the-box, the quality of image previews depends on your terminal. For the best experience with high-resolution previews, it is recommended to use a modern terminal such as:

*   [Kitty]

*   [Ghostty]

*   [Konsole]

*   [WezTerm] 
    (Requires enabling the feature, e.g., by setting enable_kitty_graphics=true in its config, and its implementation may have some known conformance issues compared to Kitty).

*   [Tabby]

Other terminals , that supports sixel would work a bit blurry , but decently great if enabled option of true colors. Treminals such as:

*   [MLTerm]

*   [Contour]

*   [Foot (Wayland-native)]

*   [WezTerm]

*   [Konsole (KDE environment)]

*   [Rio]

*   [Alacritty (Support available in some recent or experimental builds)]

## Usage

Run the application with the `karu` command:

```
karu
```

## Keybindings

| Key                 | Action                       |
| ------------------- | ---------------------------- |
| `q` / `Quit`        | Quit                         |
| `j` / `Down`        | Move down                    |
| `k` / `Up`          | Move up                      |
| `h` / `Left`        | Go up a directory            |
| `l` / `Right`       | Open file or directory       |
| `Enter`             | Open file or directory       |
| `d` / `Delete`      | Delete (move to trash)       |
| `c`                 | Copy                         |
| `x`                 | Cut                          |
| `f`                 | Paste                        |
| `n`                 | Create new file              |
| `+`                 | Create new directory         |
| `r`                 | Rename                       |
| `m`                 | Move                         |
| `o`                 | Open with default application|
| `Shift+H`           | Toggle hidden files          |
| `/`                 | Edit address bar             |
| `f`                 | Filter files                 |
| `Esc`               | Cancel action                |

## Troubleshooting

If you encounter issues while building or running `karu` on your system, it may be due to platform-specific compilation requirements or missing dependencies that are not yet documented.

When building from source on Windows or macOS, ensure you have the standard build tools configured for Rust. For Windows, this typically means having the "C++ build tools" from Visual Studio installed. For macOS, the "Command Line Tools for Xcode" are required.

If the application fails to build or run, please open an issue on the project's GitHub page. Include the following details in your report:

1.  **Operating System:** (e.g., Windows 11, macOS 12.5)
2.  **Command:** The command you tried to execute (e.g., `cargo build --release --verbose`).
3.  **Output:** The full, unedited output or error message from the command.

This information is crucial for diagnosing and fixing cross-platform issues.

## Contributing

Contributions are welcome! Please feel free to submit a pull request or open an issue.

## License

This project is not explicitly licensed.
