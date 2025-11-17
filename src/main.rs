use anyhow::{Context, Result};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use nerd_font_symbols::md;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};
use std::{
    env,
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};
use trash;
use viuer;
use open;

const ACTIONS: &[(&str, &str)] = &[
    ("Cut", "X"),
    ("Copy", "C"),
    ("Paste", "P"),
    ("Delete", "D"),
    ("Rename", "R"),
    ("Create", "N"),
    ("Create Directory", "+"),
    ("Move", "M"),
    ("Open", "O"),
    ("Toggle Hidden", "Shift+H"),
];
const VIM_KEY_HINTS: &[(&str, &str, &str)] = &[
    ("j", "Down Arrow", "Move down in file list"),
    ("k", "Up Arrow", "Move up in file list"),
    ("h", "Left Arrow", "Unfocus actions panel / Go up directory"),
    ("l", "Right Arrow", "Focus actions panel / Open selected"),
    ("q", "Quit", "Quit the application"),
];

#[derive(PartialEq)]
enum AppMode {
    Normal,
    ConfirmDelete,
    Editing,
    Create,
    Rename,
    Filter,
    CreateDirectory,
    Move,
}
#[derive(PartialEq)]
enum PanelFocus {
    Files,
    Actions,
}
struct App {
    path: PathBuf,
    files: Vec<String>,
    selected: usize,
    mode: AppMode,
    address_input: String,
    cursor_position: usize,
    create_input: String,
    rename_input: String,
    clipboard: Option<PathBuf>,
    is_cut: bool,
    show_hidden: bool,
    filter_input: String,
    create_directory_input: String,
    move_input: String,
    selected_action: usize,
    panel_focus: PanelFocus,
    action_list_state: ListState,
    error_message: Option<String>,
}
impl App {
    fn new(path: PathBuf) -> Result<Self> {
        let normalized_path = Self::normalize_path(&path)?;
        let files = Self::get_files(&normalized_path, true)?;
        let address_input = normalized_path
            .to_str()
            .context("Invalid path")?
            .to_string();
        let cursor_position = address_input.len();
        Ok(Self {
            path: normalized_path,
            files,
            selected: 0,
            mode: AppMode::Normal,
            address_input,
            cursor_position,
            create_input: String::new(),
            rename_input: String::new(),
            clipboard: None,
            is_cut: false,
            show_hidden: true,
            filter_input: String::new(),
            create_directory_input: String::new(),
            move_input: String::new(),
            selected_action: 0,
            panel_focus: PanelFocus::Files,
            action_list_state: ListState::default(),
            error_message: None,
        })
    }
    fn normalize_path(path: &Path) -> Result<PathBuf> {
        if path.starts_with("~") {
            let home = env::var("HOME").context("Failed to get HOME directory")?;
            let mut new_path = PathBuf::new();
            new_path.push(home);
            new_path.push(path.strip_prefix("~").expect("Path starts with ~"));
            Ok(new_path)
        } else {
            Ok(path.to_path_buf())
        }
    }
    fn get_files(path: &Path, show_hidden: bool) -> Result<Vec<String>> {
        let mut all_entries: Vec<PathBuf> = fs::read_dir(path)?
            .filter_map(|res| res.ok().map(|e| e.path()))
            .collect();
        all_entries.sort_by(|a, b| {
            a.file_name()
                .unwrap_or_default()
                .to_ascii_lowercase()
                .cmp(&b.file_name().unwrap_or_default().to_ascii_lowercase())
        });
        let mut hidden_dirs = Vec::new();
        let mut normal_dirs = Vec::new();
        let mut hidden_files = Vec::new();
        let mut normal_files = Vec::new();
        for entry_path in all_entries {
            let file_name = entry_path
                .file_name()
                .context("Failed to get file name")?
                .to_string_lossy()
                .to_string();
            if file_name == "." || file_name == ".." {
                continue;
            }
            let is_hidden = file_name.starts_with('.');
            let is_dir = entry_path.is_dir();
            if is_hidden && !show_hidden {
                continue;
            }
            if is_dir {
                if is_hidden {
                    hidden_dirs.push(file_name);
                } else {
                    normal_dirs.push(file_name);
                }
            } else {
                if is_hidden {
                    hidden_files.push(file_name);
                } else {
                    normal_files.push(file_name);
                }
            }
        }
        let mut files = vec!["..".to_string()];
        files.extend(hidden_dirs);
        files.extend(normal_dirs);
        files.extend(hidden_files);
        files.extend(normal_files);
        Ok(files)
    }
    fn select_next(&mut self) {
        if self.selected < self.files.len() - 1 {
            self.selected += 1;
        }
    }
    fn select_previous(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }
    fn open_selected(&mut self) -> Result<()> {
        let selected_file = &self.files[self.selected];
        if selected_file == ".." {
            self.go_up_directory()?;
            return Ok(())
        }
        let new_path = self.path.join(selected_file);
        let normalized_path = Self::normalize_path(&new_path)?;
        if normalized_path.is_dir() {
            self.path = normalized_path;
            self.files = Self::get_files(&self.path, self.show_hidden)?;
            self.selected = 0;
        } else {
            open::that(&normalized_path)?;
        }
        Ok(())
    }
    fn delete_selected(&mut self) {
        self.mode = AppMode::ConfirmDelete;
    }
    fn confirm_delete(&mut self) -> Result<()> {
        let selected_file = &self.files[self.selected];
        let path = self.path.join(selected_file);
        trash::delete(path)?;
        self.files = Self::get_files(&self.path, self.show_hidden)?;
        self.selected = 0;
        self.mode = AppMode::Normal;
        Ok(())
    }
    fn cancel_delete(&mut self) {
        self.mode = AppMode::Normal;
    }
    fn copy_selected(&mut self) {
        let selected_file = &self.files[self.selected];
        let path = self.path.join(selected_file);
        self.clipboard = Some(path);
    }
    fn cut_selected(&mut self) {
        let selected_file = &self.files[self.selected];
        let path = self.path.join(selected_file);
        self.clipboard = Some(path);
        self.is_cut = true;
        self.mode = AppMode::Normal;
    }
    fn paste(&mut self) -> Result<()> {
        if let Some(from) = self.clipboard.clone() {
            let to = self
                .path
                .join(from.file_name().context("Failed to get file name")?);
            if from.is_dir() {
                fs::create_dir_all(&to)?;
                for entry in fs::read_dir(from.clone())? {
                    let entry = entry?;
                    let path = entry.path();
                    let to = to.join(path.file_name().context("Failed to get file name")?);
                    fs::copy(path, to)?;
                }
            } else {
                fs::copy(&from, &to)?;
            }
            if self.is_cut {
                if from.is_dir() {
                    fs::remove_dir_all(&from)?;
                } else {
                    fs::remove_file(&from)?;
                }
                self.is_cut = false;
                self.clipboard = None;
            }
            self.files = Self::get_files(&self.path, self.show_hidden)?;
        }
        Ok(())
    }

    fn open_file(&mut self) -> Result<()> {
        let selected_file = &self.files[self.selected];
        let path = self.path.join(selected_file);
        if !path.is_dir() {
            open::that(&path)?;
        }
        Ok(())
    }
    fn toggle_hidden_files(&mut self) -> Result<()> {
        self.show_hidden = !self.show_hidden;
        self.files = Self::get_files(&self.path, self.show_hidden)?;
        self.selected = 0;
        Ok(())
    }
    fn go_up_directory(&mut self) -> Result<()> {
        let parent = self.path.parent().context("Already at root")?;
        self.path = parent.to_path_buf();
        self.files = Self::get_files(&self.path, self.show_hidden)?;
        self.selected = 0;
        Ok(())
    }
}
fn ui(f: &mut Frame, app: &mut App) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(f.area());
    let address_bar = render_address_bar(app);
    f.render_widget(address_bar, main_chunks[0]);
    if app.mode == AppMode::Editing {
        f.set_cursor_position(Position::new(
            main_chunks[0].x + app.cursor_position as u16 + 1,
            main_chunks[0].y + 1,
        ));
    }
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
        .split(main_chunks[1]);
    let file_list_width = content_chunks[0].width;
    let file_list = render_file_list(app, file_list_width, &app.panel_focus);
    let mut state = ListState::default();
    state.select(Some(app.selected));
    f.render_stateful_widget(file_list, content_chunks[0], &mut state);
    let right_chunks = Layout::default()
        .constraints([Constraint::Percentage(35), Constraint::Percentage(70)].as_ref())
        .split(content_chunks[1]);
    let context_menu = render_context_menu(&app.panel_focus);
    app.action_list_state.select(Some(app.selected_action));
    f.render_stateful_widget(context_menu, right_chunks[0], &mut app.action_list_state);

    let right_panel_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)].as_ref())
        .split(right_chunks[1]);

    render_preview(f, app, right_panel_chunks[0]);
    render_key_hints(f, right_panel_chunks[1]);

    if let Some(error_message) = &app.error_message {
        let area = centered_rect(60, 20, f.area());
        let p = Paragraph::new(error_message.as_str())
            .block(Block::default().title("Error").borders(Borders::ALL))
            .wrap(ratatui::widgets::Wrap { trim: true });
        f.render_widget(Clear, area);
        f.render_widget(p, area);
    }
    if let AppMode::ConfirmDelete = app.mode {
        let block = Block::default()
            .title("Confirm Delete")
            .borders(Borders::ALL);
        let area = centered_rect(60, 20, f.area());
        f.render_widget(Clear, area);
        f.render_widget(block, area);
        let p = Paragraph::new("Are you sure you want to move to trash? (y/n)");
        f.render_widget(p, area);
    }
    if let AppMode::Create = app.mode {
        let block = Block::default().title("Create New").borders(Borders::ALL);
        let area = centered_rect(60, 20, f.area());
        f.render_widget(Clear, area);
        f.render_widget(block, area);
        let p = Paragraph::new(app.create_input.as_str());
        f.render_widget(p, area);
        f.set_cursor_position(Position::new(
            area.x + app.create_input.len() as u16 + 1,
            area.y + 1,
        ));
    }
    if let AppMode::Rename = app.mode {
        let block = Block::default().title("Rename").borders(Borders::ALL);
        let area = centered_rect(60, 20, f.area());
        f.render_widget(Clear, area);
        f.render_widget(block, area);
        let p = Paragraph::new(app.rename_input.as_str());
        f.render_widget(p, area);
        f.set_cursor_position(Position::new(
            area.x + app.rename_input.len() as u16 + 1,
            area.y + 1,
        ));
    }
    if let AppMode::Filter = app.mode {
        let block = Block::default().title("Filter").borders(Borders::ALL);
        let area = centered_rect(60, 20, f.area());
        f.render_widget(Clear, area);
        f.render_widget(block, area);
        let p = Paragraph::new(app.filter_input.as_str());
        f.render_widget(p, area);
        f.set_cursor_position(Position::new(
            area.x + app.filter_input.len() as u16 + 1,
            area.y + 1,
        ));
    }
    if let AppMode::CreateDirectory = app.mode {
        let block = Block::default()
            .title("Create Directory")
            .borders(Borders::ALL);
        let area = centered_rect(60, 20, f.area());
        f.render_widget(Clear, area);
        f.render_widget(block, area);
        let p = Paragraph::new(app.create_directory_input.as_str());
        f.render_widget(p, area);
        f.set_cursor_position(Position::new(
            area.x + app.create_directory_input.len() as u16 + 1,
            area.y + 1,
        ));
    }
    if let AppMode::Move = app.mode {
        let block = Block::default().title("Move").borders(Borders::ALL);
        let area = centered_rect(60, 20, f.area());
        f.render_widget(Clear, area);
        f.render_widget(block, area);
        let p = Paragraph::new(app.move_input.as_str());
        f.render_widget(p, area);
        f.set_cursor_position(Position::new(
            area.x + app.move_input.len() as u16 + 1,
            area.y + 1,
        ));
}
}
fn render_key_hints(f: &mut Frame, area: Rect) {
    let mut spans = Vec::new();
    for (vim_key, arrow_key, _description) in VIM_KEY_HINTS.iter() { // Ignore description
        spans.push(Span::styled(format!("{vim_key}"), Style::default().fg(Color::Yellow)));
        spans.push(Span::raw("/"));
        spans.push(Span::styled(format!("{arrow_key:<10}"), Style::default().fg(Color::Cyan)));
        spans.push(Span::raw("  ")); // Add some spacing between hints
    }

    let paragraph = Paragraph::new(Line::from(spans))
        .block(
            Block::default()
                .title("Key Hints")
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Reset)),
        )
        .alignment(Alignment::Center); // Center the text for better appearance
    f.render_widget(paragraph, area);
}

fn render_address_bar<'a>(app: &'a App) -> Paragraph<'a> {
    let path_str = if app.mode == AppMode::Editing {
        app.address_input.as_str()
    } else {
        app.path.to_str().unwrap_or("Karu")
    };
    Paragraph::new(path_str).block(Block::default().title("Address").borders(Borders::ALL))
}
fn render_file_list<'a>(app: &'a App, max_width: u16, panel_focus: &PanelFocus) -> List<'a> {
    let items: Vec<ListItem> = app
        .files
        .iter()
        .map(|i| {
            let path = app.path.join(i);
            let is_dir = path.is_dir();
            let color = if is_dir {
                Color::Rgb(0, 200, 128) // Dark Green
            } else {
                Color::Blue
            };
            let style = Style::default().fg(color);

            let glyph = if is_dir {
                md::MD_FOLDER_OPEN
            } else {
                md::MD_FILE
            };

            let size_width = 10;
            let name_width = (max_width as usize).saturating_sub(size_width + 4);

            let display_name_str = if i.chars().count() > name_width {
                i.chars().take(name_width - 3).collect::<String>() + "..."
            } else {
                i.clone()
            };

            let name_part_width =
                glyph.trim().chars().count() + 2 + display_name_str.chars().count();
            let padding_width = name_width.saturating_sub(name_part_width);
            let padding = " ".repeat(padding_width);

            let mut spans = vec![
                Span::styled(glyph.trim(), style),
                Span::styled(format!("  {display_name_str}"), style),
                Span::raw(padding),
            ];

            if !is_dir {
                if let Ok(metadata) = fs::metadata(&path) {
                    let size = metadata.len();
                    let formatted_size = format_size(size);
                    let padded_size = format!("{:>width$}", formatted_size, width = size_width);
                    spans.push(Span::raw(padded_size));
                }
            }

            ListItem::new(Line::from(spans))
        })
        .collect();
    let mut list = List::new(items).block(Block::default().title("Files").borders(Borders::ALL));
    list = list.highlight_style(Style::default().bg(Color::Rgb(70, 70, 70))); // A subtle background for selected item when not focused

    if let PanelFocus::Files = panel_focus {
        list = list
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(70, 70, 70))
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");
    }
    list
}
fn render_context_menu(panel_focus: &PanelFocus) -> List<'_> {
    let items: Vec<ListItem> = ACTIONS
        .iter()
        .map(|(action, shortcut)| ListItem::new(format!("{} ({})", action, shortcut)))
        .collect();
    let mut list = List::new(items)
        .block(
            Block::default()
                .title("Actions")
                .borders(Borders::ALL),
        );
    if let PanelFocus::Actions = panel_focus {
        list = list
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(50, 50, 50))
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");
    }
    list
}
fn render_preview(f: &mut Frame, app: &mut App, area: Rect) {
    f.render_widget(Clear, area);
    let path = if let Some(selected_file) = app.files.get(app.selected) {
        app.path.join(selected_file)
    } else {
        return;
    };

    // Explicitly block .wget-hsts file
    if path.file_name().map_or(false, |name| name == ".wget-hsts") {
        let p = Paragraph::new("'.wget-hsts' file is blocked from preview.")
            .block(Block::default().title("Preview").borders(Borders::ALL));
        f.render_widget(p, area);
        return; // Exit the function early
    }

    // Check file size for preview
    if let Ok(metadata) = fs::metadata(&path) {
        const MAX_PREVIEW_SIZE_MB: u64 = 300;
        const MAX_PREVIEW_SIZE_BYTES: u64 = MAX_PREVIEW_SIZE_MB * 1024 * 1024; // 300 MB in bytes
        if metadata.len() > MAX_PREVIEW_SIZE_BYTES {
            let p = Paragraph::new(format!(
                "File is too large for preview ({}) Max size is {} MB.",
                format_size(metadata.len()),
                MAX_PREVIEW_SIZE_MB
            ))
            .block(Block::default().title("Preview").borders(Borders::ALL));
            f.render_widget(p, area);
            return; // Exit the function early
        }
    }
    if is_image(&path) {
        if let Ok(_img) = image::open(&path) {
            let inner_area = area.inner(Margin {
                horizontal: 1,
                vertical: 1,
            });
            let config = viuer::Config {
                x: inner_area.x,
                y: inner_area.y as i16,
                width: Some(inner_area.width as u32),
                height: Some(inner_area.height as u32),
                ..Default::default()
            };
            viuer::print_from_file(path, &config).expect("Image printing failed.");
            // Draw the block and borders after the image to make them visible
            let block = Block::default().title("Preview").borders(Borders::ALL).style(Style::default().bg(Color::Reset));
            f.render_widget(block, area);
        } else {
            let p = Paragraph::new("Could not load image")
                .block(Block::default().title("Preview").borders(Borders::ALL));
            f.render_widget(p, area);
        }
    } else if is_likely_binary(&path) {
        let p = Paragraph::new("Binary file, no preview available.")
            .block(Block::default().title("Preview").borders(Borders::ALL));
        f.render_widget(p, area);
    } else {
        let block = Block::default().style(Style::default().bg(Color::Reset));
        f.render_widget(block, area);

        let content = if path.is_dir() {
            "Directory".to_string()
        } else {
            fs::read_to_string(path).unwrap_or_else(|err| format!("Cannot read file: {}", err))
        };
        let max_width = area.width.saturating_sub(2) as usize;
        let truncated_content: String = content
            .lines()
            .map(|line| {
                if line.len() > max_width {
                    format!("{}\"...", &line[0..max_width.saturating_sub(3)])
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<String>>()
            .join("\n");
        let p = Paragraph::new(truncated_content)
            .block(Block::default().title("Preview").borders(Borders::ALL))
            .style(Style::default().bg(Color::Reset));
        f.render_widget(p, area);
    }
}
fn is_image(path: &Path) -> bool {
    let extension = path.extension().and_then(|s| s.to_str());
    if let Some(ext) = extension {
        matches!(
            ext.to_lowercase().as_str(),
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "tiff" | "webp"
        )
    } else {
        false
    }
}

fn is_likely_binary(path: &Path) -> bool {
    if path.is_dir() {
        return false;
    }
    let mut file = match fs::File::open(path) {
        Ok(file) => file,
        Err(_) => return false,
    };
    let mut buffer = [0; 1024];
    let n = match file.read(&mut buffer) {
        Ok(n) => n,
        Err(_) => return false,
    };
    for &byte in &buffer[..n] {
        if byte == 0 {
            return true;
        }
    }
    false
}
fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if size >= GB {
        format!("{:.1} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.1} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.1} KB", size as f64 / KB as f64)
    } else {
        format!("{} B", size)
    }
}
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}
fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if crossterm::event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                // Handle universal quit key
                if key.code == KeyCode::Char('q') {
                    return Ok(());
                }

                if let Some(_) = &app.error_message {
                    if let KeyCode::Enter | KeyCode::Esc = key.code {
                        app.error_message = None;
                    }
                    continue;
                }
                let result = match app.mode {
                    AppMode::Normal => {
                        match app.panel_focus {
                            PanelFocus::Files => match key.code {
                                KeyCode::Down | KeyCode::Char('j') => {
                                    app.select_next();
                                    Ok(())
                                }
                                KeyCode::Up | KeyCode::Char('k') => {
                                    app.select_previous();
                                    Ok(())
                                }
                                KeyCode::Enter => app.open_selected(),
                                KeyCode::Char('u') => app.go_up_directory(),
                                KeyCode::Char('d') => {
                                    app.delete_selected();
                                    Ok(())
                                }
                                KeyCode::Char('/') => {
                                    app.mode = AppMode::Editing;
                                    Ok(())
                                }
                                KeyCode::Char('n') => {
                                    app.mode = AppMode::Create;
                                    Ok(())
                                }
                                KeyCode::Char('c') => {
                                    app.copy_selected();
                                    Ok(())
                                }
                                KeyCode::Char('x') => {
                                    app.cut_selected();
                                    Ok(())
                                }
                                KeyCode::Char('p') => app.paste(),

                                KeyCode::Char('o') => app.open_file(),
                                KeyCode::Char('H')
                                    if key.modifiers.contains(KeyModifiers::SHIFT) =>
                                {
                                    app.toggle_hidden_files()
                                }
                                KeyCode::Char('f') => {
                                    app.mode = AppMode::Filter;
                                    Ok(())
                                }
                                KeyCode::Char('r') => {
                                    app.mode = AppMode::Rename;
                                    Ok(())
                                }
                                KeyCode::Char('+') => {
                                    app.mode = AppMode::CreateDirectory;
                                    Ok(())
                                }
                                KeyCode::Delete => {
                                    app.delete_selected();
                                    Ok(())
                                }
                                KeyCode::Char('m') => {
                                    app.mode = AppMode::Move;
                                    Ok(())
                                }
                                KeyCode::Right | KeyCode::Char('l') => {
                                    app.panel_focus = PanelFocus::Actions;
                                    Ok(())
                                }
                                _ => Ok(()), // Ignore other keys
                            },
                            PanelFocus::Actions => {
                                match key.code {
                                    KeyCode::Up => {
                                        if app.selected_action > 0 {
                                            app.selected_action -= 1;
                                            app.action_list_state
                                                .select(Some(app.selected_action));
                                        }
                                    }
                                    KeyCode::Down => {
                                        if app.selected_action < ACTIONS.len() - 1 {
                                            app.selected_action += 1;
                                            app.action_list_state
                                                .select(Some(app.selected_action));
                                        }
                                    }
                                    KeyCode::Left | KeyCode::Char('h') => {
                                        app.panel_focus = PanelFocus::Files
                                    }
                                    KeyCode::Enter => {
                                        match app.selected_action {
                                            0 => app.cut_selected(),
                                            1 => app.copy_selected(),
                                            2 => {
                                                if let Err(e) = app.paste() {
                                                    app.error_message = Some(e.to_string())
                                                }
                                            }
                                            3 => app.delete_selected(),
                                            4 => app.mode = AppMode::Rename,
                                            5 => app.mode = AppMode::Create,
                                            6 => app.mode = AppMode::CreateDirectory,
                                            7 => app.mode = AppMode::Move,
                                            8 => {
                                                if let Err(e) = app.open_file() {
                                                    app.error_message = Some(e.to_string())
                                                }
                                            }
                                            9 => {
                                                if let Err(e) = app.toggle_hidden_files() {
                                                    app.error_message = Some(e.to_string())
                                                }
                                            }
                                            _ => {}
                                        }
                                        app.mode = AppMode::Normal; // Return to normal mode after action
                                        app.panel_focus = PanelFocus::Files; // Return focus to files panel
                                    }
                                    KeyCode::Esc => {
                                        app.mode = AppMode::Normal;
                                        app.panel_focus = PanelFocus::Files; // Return focus to files panel
                                    }
                                    _ => {} // Ignore other keys
                                }
                                Ok(())
                            }
                        }
                    }
                    AppMode::ConfirmDelete => match key.code {
                        KeyCode::Char('y') => app.confirm_delete(),
                        KeyCode::Char('n') => {
                            app.cancel_delete();
                            Ok(())
                        }
                        _ => Ok(()), // Ignore other keys
                    },
                    AppMode::Editing => match key.code {
                        KeyCode::Char(c) => {
                            app.address_input.insert(app.cursor_position, c);
                            app.cursor_position += 1;
                            Ok(())
                        }
                        KeyCode::Backspace => {
                            if app.cursor_position > 0 {
                                app.cursor_position -= 1;
                                app.address_input.remove(app.cursor_position);
                            }
                            Ok(())
                        }
                        KeyCode::Enter => {
                            let new_path = PathBuf::from(&app.address_input);
                            if new_path.is_dir() {
                                app.path = new_path;
                                app.files = App::get_files(&app.path, app.show_hidden)?;
                                app.selected = 0;
                            }
                            app.mode = AppMode::Normal;
                            Ok(())
                        }
                        KeyCode::Esc => {
                            app.mode = AppMode::Normal;
                            Ok(())
                        }
                        _ => Ok(()), // Ignore other keys
                    },
                    AppMode::Create => match key.code {
                        KeyCode::Char(c) => {
                            app.create_input.push(c);
                            Ok(())
                        }
                        KeyCode::Backspace => {
                            app.create_input.pop();
                            Ok(())
                        }
                        KeyCode::Enter => {
                            let new_path = app.path.join(&app.create_input);
                            if new_path.ends_with("/") {
                                fs::create_dir_all(new_path)?;
                            } else {
                                fs::File::create(new_path)?;
                            }
                            app.files = App::get_files(&app.path, app.show_hidden)?;
                            app.create_input.clear();
                            app.mode = AppMode::Normal;
                            Ok(())
                        }
                        KeyCode::Esc => {
                            app.create_input.clear();
                            app.mode = AppMode::Normal;
                            Ok(())
                        }
                        _ => Ok(()), // Ignore other keys
                    },
                    AppMode::Rename => match key.code {
                        KeyCode::Char(c) => {
                            app.rename_input.push(c);
                            Ok(())
                        }
                        KeyCode::Backspace => {
                            app.rename_input.pop();
                            Ok(())
                        }
                        KeyCode::Enter => {
                            let old_path = app.path.join(&app.files[app.selected]);
                            let new_path = app.path.join(&app.rename_input);
                            fs::rename(old_path, new_path)?;
                            app.files = App::get_files(&app.path, app.show_hidden)?;
                            app.rename_input.clear();
                            app.mode = AppMode::Normal;
                            Ok(())
                        }
                        KeyCode::Esc => {
                            app.rename_input.clear();
                            app.mode = AppMode::Normal;
                            Ok(())
                        }
                        _ => Ok(()), // Ignore other keys
                    },
                    AppMode::Filter => match key.code {
                        KeyCode::Char(c) => {
                            app.filter_input.push(c);
                            Ok(())
                        }
                        KeyCode::Backspace => {
                            app.filter_input.pop();
                            Ok(())
                        }
                        KeyCode::Enter => {
                            app.files = App::get_files(&app.path, app.show_hidden)?;
                            app.files.retain(|f| f.contains(&app.filter_input));
                            app.selected = 0;
                            app.mode = AppMode::Normal;
                            Ok(())
                        }
                        KeyCode::Esc => {
                            app.filter_input.clear();
                            app.files = App::get_files(&app.path, app.show_hidden)?;
                            app.mode = AppMode::Normal;
                            Ok(())
                        }
                        _ => Ok(()), // Ignore other keys
                    },
                    AppMode::CreateDirectory => match key.code {
                        KeyCode::Char(c) => {
                            app.create_directory_input.push(c);
                            Ok(())
                        }
                        KeyCode::Backspace => {
                            app.create_directory_input.pop();
                            Ok(())
                        }
                        KeyCode::Enter => {
                            let new_path = app.path.join(&app.create_directory_input);
                            fs::create_dir_all(new_path)?;
                            app.files = App::get_files(&app.path, app.show_hidden)?;
                            app.create_directory_input.clear();
                            app.mode = AppMode::Normal;
                            Ok(())
                        }
                        KeyCode::Esc => {
                            app.create_directory_input.clear();
                            app.mode = AppMode::Normal;
                            Ok(())
                        }
                        _ => Ok(()), // Ignore other keys
                    },
                    AppMode::Move => match key.code {
                        KeyCode::Char(c) => {
                            app.move_input.push(c);
                            Ok(())
                        }
                        KeyCode::Backspace => {
                            app.move_input.pop();
                            Ok(())
                        }
                        KeyCode::Enter => {
                            let old_path = app.path.join(&app.files[app.selected]);
                            let new_path = PathBuf::from(&app.move_input);
                            fs::rename(old_path, new_path)?;
                            app.files = App::get_files(&app.path, app.show_hidden)?;
                            app.move_input.clear();
                            app.mode = AppMode::Normal;
                            Ok(())
                        }
                        KeyCode::Esc => {
                            app.move_input.clear();
                            app.mode = AppMode::Normal;
                            Ok(())
                        }
                        _ => Ok(()),
                    },
                };
                if let Err(e) = result {
                    app.error_message = Some(e.to_string());
                }
            }
        }
    }
}
fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut app = App::new(env::current_dir()?)?;
    let res = run_app(&mut terminal, &mut app);
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    if let Err(err) = res {
        println!("{err:?}");
    }
    Ok(())
}
