use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use image;
use nerd_font_symbols::md;
use ratatui::{
    prelude::*,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};
use std::{
    env,
    error::Error,
    fs,
    io::{self},
    path::{Path, PathBuf},
    process::Stdio,
};
use trash;
use unicode_width;
use viuer;
#[derive(PartialEq)]
enum AppMode {
    Normal,
    ConfirmDelete,
    Editing,
    Create,
    Rename,
    Filter,
    View,
    Edit,
    CreateDirectory,
    Move,
    Find,
    Replace,
}
#[derive(PartialEq)]
enum PanelFocus {
    Files,
    Actions,
    TopBarButtons,
}
#[derive(PartialEq)]
enum TopBarFocus {
    AddressBar,
    PrevButton,
    NextButton,
    UpButton,
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
    file_content: String,
    edit_content: String,
    create_directory_input: String,
    move_input: String,
    find_input: String,
    replace_input: String,
    selected_action: usize,
    panel_focus: PanelFocus,
    action_list_state: ListState,
    history: Vec<PathBuf>,
    history_index: usize,
    top_bar_focus: TopBarFocus,
}
impl App {
    fn new(path: PathBuf) -> Self {
        let normalized_path = Self::normalize_path(&path);
        let files = Self::get_files(&normalized_path, true);
        let address_input = normalized_path.to_str().unwrap().to_string();
        let cursor_position = address_input.len();
        Self {
            path: normalized_path.clone(),files,selected: 0,mode: AppMode::Normal,address_input,cursor_position,create_input: String::new(),rename_input: String::new(),clipboard: None,is_cut: false,show_hidden: true,filter_input: String::new(),file_content: String::new(),edit_content: String::new(),create_directory_input: String::new(),move_input: String::new(),find_input: String::new(),replace_input: String::new(),selected_action: 0,panel_focus: PanelFocus::Files,action_list_state: ListState::default(),history: vec![normalized_path.clone()],history_index: 0,top_bar_focus: TopBarFocus::AddressBar,}
    }
    fn normalize_path(path: &Path) -> PathBuf {
        if path.starts_with("~") {
            PathBuf::from(env::var("HOME").unwrap()).join(path.strip_prefix("~").unwrap())
        } else {
            path.to_path_buf()
        }
    }
    fn get_files(path: &Path, show_hidden: bool) -> Vec<String> {
        let mut all_entries: Vec<PathBuf> = fs::read_dir(path)
            .unwrap()
            .filter_map(|res| res.ok().map(|e| e.path()))
            .collect();
        all_entries.sort_by(|a, b| {
            let a_name = a.file_name().unwrap().to_string_lossy();
            let b_name = b.file_name().unwrap().to_string_lossy();

            let a_has_non_ascii = a_name.chars().any(|c| !c.is_ascii());
            let b_has_non_ascii = b_name.chars().any(|c| !c.is_ascii());

            match (a_has_non_ascii, b_has_non_ascii) {
                (true, false) => std::cmp::Ordering::Less,    (false, true) => std::cmp::Ordering::Greater,    _ => match (a_name.starts_with('.'), b_name.starts_with('.')) {
                    (true, false) => std::cmp::Ordering::Greater,        (false, true) => std::cmp::Ordering::Less,        _ => a_name.to_lowercase().cmp(&b_name.to_lowercase()),    },}
        });
        let (mut hidden_dirs, mut normal_dirs, mut hidden_files, mut normal_files) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for entry_path in all_entries {
            let file_name = entry_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string();
            if file_name == "." || file_name == ".." {
                continue;
            }
            let is_hidden = file_name.starts_with('.');
            if !show_hidden && is_hidden {
                continue;
            }
            if entry_path.is_dir() {
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
        files
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
    fn open_selected(&mut self) {
        let selected_file = &self.files[self.selected];
        if selected_file == ".." {
            let parent = self.path.parent().unwrap_or(&self.path);
            self.path = parent.to_path_buf();
            self.files = Self::get_files(&self.path, self.show_hidden);
            self.selected = 0;
            return;
        }
        let new_path = self.path.join(selected_file);
        let normalized_path = Self::normalize_path(&new_path);
        if normalized_path.is_dir() {
            self.update_path(normalized_path);
        } else {
            let _ = std::process::Command::new("xdg-open")
                .arg(&normalized_path)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
        }
    }
    fn go_to_parent_directory(&mut self) {
        if let Some(parent) = self.path.parent() {
            self.update_path(parent.to_path_buf());
        }
    }
    fn go_back(&mut self) {
        if self.history_index > 0 {
            self.history_index -= 1;
            self.path = self.history[self.history_index].clone();
            self.files = Self::get_files(&self.path, self.show_hidden);
            self.selected = 0;
        }
    }
    fn go_forward(&mut self) {
        if self.history_index < self.history.len() - 1 {
            self.history_index += 1;
            self.path = self.history[self.history_index].clone();
            self.files = Self::get_files(&self.path, self.show_hidden);
            self.selected = 0;
        }
    }
    fn update_path(&mut self, new_path: PathBuf) {
        if self.history_index < self.history.len() - 1 {
            self.history.truncate(self.history_index + 1);
        }
        self.path = new_path.clone();
        self.files = Self::get_files(&self.path, self.show_hidden);
        self.selected = 0;
        self.history.push(new_path);
        self.history_index = self.history.len() - 1;
    }
    fn delete_selected(&mut self) {
        self.mode = AppMode::ConfirmDelete;
    }
    fn confirm_delete(&mut self) {
        let selected_file = &self.files[self.selected];
        let path = self.path.join(selected_file);
        trash::delete(path).unwrap();
        self.files = Self::get_files(&self.path, self.show_hidden);
        self.selected = 0;
        self.mode = AppMode::Normal;
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
    fn paste(&mut self) {
        if let Some(from) = self.clipboard.clone() {
            let to = self.path.join(from.file_name().unwrap());
            if from.is_dir() {
                fs::create_dir_all(&to).unwrap();
                for entry in fs::read_dir(from.clone()).unwrap() {
                    let entry = entry.unwrap();
                    let path = entry.path();
                    let to = to.join(path.file_name().unwrap());
                    fs::copy(path, to).unwrap();
                }
            } else {
                fs::copy(&from, &to).unwrap();
            }
            if self.is_cut {
                if from.is_dir() {
                    fs::remove_dir_all(&from).unwrap();
                } else {
                    fs::remove_file(&from).unwrap();
                }
                self.is_cut = false;
                self.clipboard = None;
            }
            self.files = Self::get_files(&self.path, self.show_hidden);
        }
    }
    fn save_file(&mut self) {
        let selected_file = &self.files[self.selected];
        let path = self.path.join(selected_file);
        if !path.is_dir() {
            let content =
                fs::read_to_string(path).unwrap_or_else(|_| "Cannot read file".to_string());
            fs::write("saved_file.txt", content).unwrap();
        }
    }
    fn open_file(&mut self) {
        let selected_file = &self.files[self.selected];
        let path = self.path.join(selected_file);
        if !path.is_dir() {
            let _ = std::process::Command::new("xdg-open")
                .arg(path)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
        }
    }
    fn toggle_hidden_files(&mut self) {
        self.show_hidden = !self.show_hidden;
        self.files = Self::get_files(&self.path, self.show_hidden);
        self.selected = 0;
    }
    fn view_file(&mut self) {
        let selected_file = &self.files[self.selected];
        let path = self.path.join(selected_file);
        if !path.is_dir() {
            self.file_content =
                fs::read_to_string(path).unwrap_or_else(|_| "Cannot read file".to_string());
            self.mode = AppMode::View;
        }
    }
    fn edit_file(&mut self) {
        let selected_file = &self.files[self.selected];
        let path = self.path.join(selected_file);
        if !path.is_dir() {
            self.edit_content =
                fs::read_to_string(path).unwrap_or_else(|_| "Cannot read file".to_string());
            self.mode = AppMode::Edit;
        }
    }
}
fn ui(f: &mut Frame, app: &mut App) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(f.size());
    let top_bar_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(20)].as_ref())
        .split(main_chunks[0]);
    let mut address_bar_block = Block::default().title("Address").borders(Borders::ALL);
    if app.panel_focus == PanelFocus::TopBarButtons && app.top_bar_focus == TopBarFocus::AddressBar
    {
        address_bar_block = address_bar_block.border_style(Style::default().fg(Color::Cyan));
    }
    let address_bar = render_address_bar(app).block(address_bar_block);
    f.render_widget(address_bar, top_bar_chunks[0]);
    let nav_buttons_area = top_bar_chunks[1];
    let nav_buttons_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(33),    Constraint::Percentage(33),    Constraint::Percentage(34),]
            .as_ref(),)
        .split(nav_buttons_area);
    let mut prev_button = Paragraph::new("< Prev").block(Block::default().borders(Borders::ALL));
    let mut next_button = Paragraph::new("Next >").block(Block::default().borders(Borders::ALL));
    let mut up_button = Paragraph::new("Up ^").block(Block::default().borders(Borders::ALL));
    if app.panel_focus == PanelFocus::TopBarButtons {
        match app.top_bar_focus {
            TopBarFocus::PrevButton => {
                prev_button = prev_button.block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Cyan)),    )
            }
            TopBarFocus::NextButton => {
                next_button = next_button.block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Cyan)),    )
            }
            TopBarFocus::UpButton => {
                up_button = up_button.block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Cyan)),    )
            }
            _ => {}
        }
    }
    f.render_widget(prev_button, nav_buttons_chunks[0]);
    f.render_widget(next_button, nav_buttons_chunks[1]);
    f.render_widget(up_button, nav_buttons_chunks[2]);
    if app.mode == AppMode::Editing && app.top_bar_focus == TopBarFocus::AddressBar {
        f.set_cursor(
            top_bar_chunks[0].x + app.cursor_position as u16 + 1,top_bar_chunks[0].y + 1,);
    }
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)].as_ref())
        .split(main_chunks[1]);
    let file_list_width = content_chunks[0].width;
    let file_list = render_file_list(app, file_list_width);
    let mut state = ListState::default();
    state.select(Some(app.selected));
    f.render_stateful_widget(file_list, content_chunks[0], &mut state);
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
        .split(content_chunks[1]);
    let context_menu = render_context_menu(&app.panel_focus);
    app.action_list_state.select(Some(app.selected_action));
    f.render_stateful_widget(context_menu, right_chunks[0], &mut app.action_list_state);
    render_preview(f, app, right_chunks[1]);
    if let AppMode::ConfirmDelete = app.mode {
        let area = centered_rect(60, 20, f.size());
        f.render_widget(Clear, area);
        f.render_widget(
            Block::default()
                .title("Confirm Delete")
                .borders(Borders::ALL),area,);
        f.render_widget(
            Paragraph::new("Are you sure you want to move to trash? (y/n)"),area,);
    }
    if let AppMode::Create = app.mode {
        let area = centered_rect(60, 20, f.size());
        f.render_widget(Clear, area);
        f.render_widget(
            Block::default().title("Create New").borders(Borders::ALL),area,);
        f.render_widget(Paragraph::new(app.create_input.as_str()), area);
        f.set_cursor(
            area.x + app.create_input.len() as u16 + 1,area.y + 1,);
    }
    if let AppMode::Rename = app.mode {
        let area = centered_rect(60, 20, f.size());
        f.render_widget(Clear, area);
        f.render_widget(Block::default().title("Rename").borders(Borders::ALL), area);
        f.render_widget(Paragraph::new(app.rename_input.as_str()), area);
        f.set_cursor(
            area.x + app.rename_input.len() as u16 + 1,area.y + 1,);
    }
    if let AppMode::Filter = app.mode {
        let area = centered_rect(60, 20, f.size());
        f.render_widget(Clear, area);
        f.render_widget(Block::default().title("Filter").borders(Borders::ALL), area);
        f.render_widget(Paragraph::new(app.filter_input.as_str()), area);
        f.set_cursor(
            area.x + app.filter_input.len() as u16 + 1,area.y + 1,);
    }
    if let AppMode::View = app.mode {
        let area = centered_rect(80, 80, f.size());
        f.render_widget(Clear, area);
        f.render_widget(
            Block::default().title("View File").borders(Borders::ALL),area,);
        f.render_widget(Paragraph::new(app.file_content.as_str()), area);
    }
    if let AppMode::Edit = app.mode {
        let area = centered_rect(80, 80, f.size());
        f.render_widget(Clear, area);
        f.render_widget(
            Block::default().title("Edit File").borders(Borders::ALL),area,);
        f.render_widget(Paragraph::new(app.edit_content.as_str()), area);
        f.set_cursor(
            area.x + app.edit_content.len() as u16 + 1,area.y + 1,);
    }
    if let AppMode::CreateDirectory = app.mode {
        let area = centered_rect(60, 20, f.size());
        f.render_widget(Clear, area);
        f.render_widget(
            Block::default()
                .title("Create Directory")
                .borders(Borders::ALL),area,);
        f.render_widget(Paragraph::new(app.create_directory_input.as_str()), area);
        f.set_cursor(
            area.x + app.create_directory_input.len() as u16 + 1,area.y + 1,);
    }
    if let AppMode::Move = app.mode {
        let area = centered_rect(60, 20, f.size());
        f.render_widget(Clear, area);
        f.render_widget(Block::default().title("Move").borders(Borders::ALL), area);
        f.render_widget(Paragraph::new(app.move_input.as_str()), area);
        f.set_cursor(
            area.x + app.move_input.len() as u16 + 1,area.y + 1,);
    }
    if let AppMode::Find = app.mode {
        let area = centered_rect(60, 20, f.size());
        f.render_widget(Clear, area);
        f.render_widget(Block::default().title("Find").borders(Borders::ALL), area);
        f.render_widget(Paragraph::new(app.find_input.as_str()), area);
        f.set_cursor(
            area.x + app.find_input.len() as u16 + 1,area.y + 1,);
    }
    if let AppMode::Replace = app.mode {
        let area = centered_rect(60, 20, f.size());
        f.render_widget(Clear, area);
        f.render_widget(
            Block::default().title("Replace").borders(Borders::ALL),area,);
        f.render_widget(Paragraph::new(app.replace_input.as_str()), area);
        f.set_cursor(
            area.x + app.replace_input.len() as u16 + 1,area.y + 1,);
    }
}
fn truncate_filename(name: &str, max_width: usize) -> String {
    let mut current_width = 0;
    let mut truncated_string = String::new();
    let mut chars = name.chars().peekable();
    while let Some(c) = chars.next() {
        let char_width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
        if current_width + char_width > max_width {
            truncated_string.push_str("...");
            break;
        }
        truncated_string.push(c);
        current_width += char_width;
    }
    truncated_string
}
fn render_address_bar<'a>(app: &'a App) -> Paragraph<'a> {
    let path_str = if app.mode == AppMode::Editing {
        app.address_input.as_str()
    } else {
        app.path.to_str().unwrap_or("Karu")
    };
    Paragraph::new(path_str).block(Block::default().title("Address").borders(Borders::ALL))
}
fn render_file_list<'a>(app: &'a App, max_width: u16) -> List<'a> {
    let items: Vec<ListItem> = app
        .files
        .iter()
        .map(|i| {
            let path = app.path.join(i);
            let is_dir = path.is_dir();
            let (glyph, color) = if is_dir {
                (md::MD_FOLDER_OPEN, Color::Green)
            } else {
                (md::MD_FILE, Color::Blue)
            };
            let display_name = truncate_filename(i, max_width as usize - (glyph.trim().len() + 2));
            let mut spans = vec![
                Span::styled(glyph.trim(), Style::default().fg(color)),    Span::raw(format!("  {}", display_name.trim())),];
            if !is_dir {
                if let Ok(metadata) = fs::metadata(&path) {
                    let formatted_size = format_size(metadata.len());
                    let padding = (max_width as usize).saturating_sub(
                        glyph.trim().len()
                            + 2
                            + display_name.trim().len()
                            + formatted_size.len()
                            + 4,        );
                    spans.extend_from_slice(&[
                        Span::raw(" ".repeat(padding)),            Span::raw(formatted_size),        ]);
                }
            }
            ListItem::new(Line::from(spans))
        })
        .collect();
    List::new(items)
        .block(Block::default().title("Files").borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(50, 50, 50))
                .add_modifier(Modifier::BOLD),)
        .highlight_symbol("> ")
}
fn render_context_menu(panel_focus: &PanelFocus) -> List<'_> {
    const ACTIONS: &[(&str, &str)] = &[
        ("Cut", "x"),("Copy", "c"),("Paste", "p"),("Delete", "d"),("Rename", "r"),("Create", "n"),("Create Directory", "+"),("Move", "m"),("View", "v"),("Edit", "e"),("Find", "Ctrl+f"),("Replace", "Ctrl+r"),("Toggle Hidden", "h"),("Quit", "q"),
    ];
    let mut list = List::new(
        ACTIONS
            .iter()
            .map(|(action, shortcut)| ListItem::new(format!("{} ({})", action, shortcut)))
            .collect::<Vec<ListItem>>(),
    )
    .block(Block::default().title("Actions").borders(Borders::ALL));
    if let PanelFocus::Actions = panel_focus {
        list = list
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(50, 50, 50))
                    .add_modifier(Modifier::BOLD),)
            .highlight_symbol("> ");
    }
    list
}
#[allow(unused_assignments)]
fn render_preview(f: &mut Frame, app: &mut App, area: Rect) {
    f.render_widget(Clear, area);
    let selected_file = &app.files[app.selected];
    let path = app.path.join(selected_file);
    if selected_file == ".wget-hsts" {
        f.render_widget(
            Paragraph::new("Preview not available for .wget-hsts files.")
                .block(Block::default().title("Preview").borders(Borders::ALL)),area,);
        return;
    }
    if let Ok(metadata) = fs::metadata(&path) {
        const MAX_PREVIEW_SIZE_MB: u64 = 300;
        if metadata.len() > MAX_PREVIEW_SIZE_MB * 1024 * 1024 {
            f.render_widget(
                Paragraph::new(format!(
                    "File is too large for preview ({}) Max size is {} MB.",        format_size(metadata.len()),        MAX_PREVIEW_SIZE_MB
                ))
                .block(Block::default().title("Preview").borders(Borders::ALL)),    area,);
            return;
        }
    }
    if is_media(&path) {
        if is_actual_image(&path) {
            let preview_path_option = Some(path.clone());

            if let Some(p_path) = &preview_path_option {
                match fs::read(p_path) {
                    Ok(buffer) => {
                        match image::load_from_memory(&buffer) {
                            Ok(img) => {
                                let config = viuer::Config {
                                    x: area.x + 2,
                                    y: (area.y + 1) as i16,
                                    width: Some(area.width.saturating_sub(4) as u32),
                                    height: Some(area.height.saturating_sub(2) as u32),
                                    use_kitty: true,
                                    truecolor: true,
                                    ..Default::default()
                                };
                                if viuer::print(&img, &config).is_err() {
                                    let p = Paragraph::new("Image printing failed.")
                                        .block(Block::default().title("Preview").borders(Borders::ALL));
                                    f.render_widget(p, area);
                                }
                            }
                            Err(_) => {
                                let p = Paragraph::new("Could not decode image.")
                                    .block(Block::default().title("Preview").borders(Borders::ALL));
                                f.render_widget(p, area);
                            }
                        }
                    }
                    Err(_) => {
                        let p = Paragraph::new("Could not read image file.")
                            .block(Block::default().title("Preview").borders(Borders::ALL));
                        f.render_widget(p, area);
                    }
                }
            }
        } else {
            f.render_widget(
                Paragraph::new("Video previews are currently disabled.")
                    .block(Block::default().title("Preview").borders(Borders::ALL)),    area,);
        }
    } else {
        let content = if path.is_dir() {
            "Directory".to_string()
        } else {
            fs::read_to_string(path).unwrap_or_else(|_| "Cannot read file".to_string())
        };
        let max_width = area.width.saturating_sub(2) as usize;
        let truncated_content: String = content
            .lines()
            .map(|line| {
                if line.len() > max_width {
                    format!("{}", &line[0..max_width.saturating_sub(3)])
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
fn is_actual_image(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .map_or(false, |ext| {
            matches!(
                ext.to_lowercase().as_str(),    "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "tiff" | "webp"
            )
        })
}
fn is_media(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .map_or(false, |ext| {
            matches!(
                ext.to_lowercase().as_str(),    "png"
                    | "jpg"
                    | "jpeg"
                    | "gif"
                    | "bmp"
                    | "ico"
                    | "tiff"
                    | "webp"
                    | "mp4"
                    | "mkv"
                    | "avi"
                    | "mov"
                    | "webm"
            )
        })
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
                Constraint::Percentage((100 - percent_y) / 2),    Constraint::Percentage(percent_y),    Constraint::Percentage((100 - percent_y) / 2),]
            .as_ref(),)
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),    Constraint::Percentage(percent_x),    Constraint::Percentage((100 - percent_x) / 2),]
            .as_ref(),)
        .split(popup_layout[1])[1]
}
fn get_interactive_areas(f: &Frame) -> (Rect, Rect, Rect, Rect, Rect, Rect) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(f.size());
    let top_bar_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(20)].as_ref())
        .split(main_chunks[0]);
    let nav_buttons_area = top_bar_chunks[1];
    let nav_buttons_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(33),    Constraint::Percentage(33),    Constraint::Percentage(34),]
            .as_ref(),)
        .split(nav_buttons_area);
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)].as_ref())
        .split(main_chunks[1]);
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
        .split(content_chunks[1]);
    (
        top_bar_chunks[0],nav_buttons_chunks[0],nav_buttons_chunks[1],nav_buttons_chunks[2],content_chunks[0],right_chunks[0],
    )
}
fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;
        let timeout = std::time::Duration::from_millis(250);
        if crossterm::event::poll(timeout)? {
            let (
                address_bar_rect,    prev_button_rect,    next_button_rect,    up_button_rect,    file_list_rect,    action_list_rect,) = get_interactive_areas(&terminal.get_frame());
            if let Event::Key(key) = event::read()? {
                match app.mode {
                    AppMode::Normal => match app.panel_focus {
                        PanelFocus::Files => match key.code {
                            KeyCode::Left if key.modifiers.contains(KeyModifiers::ALT) => {
                                app.go_back()
                            }
                            KeyCode::Right if key.modifiers.contains(KeyModifiers::ALT) => {
                                app.go_forward()
                            }
                            KeyCode::Up if key.modifiers.contains(KeyModifiers::ALT) => {
                                app.go_to_parent_directory()
                            }
                            KeyCode::Char('q') => return Ok(()),                KeyCode::Down => app.select_next(),                KeyCode::Up => app.select_previous(),                KeyCode::Enter => app.open_selected(),                KeyCode::Char('d') => app.delete_selected(),                KeyCode::Char('/') => {
                                app.mode = AppMode::Editing;
                                app.panel_focus = PanelFocus::TopBarButtons;
                                app.top_bar_focus = TopBarFocus::AddressBar;
                            }
                            KeyCode::Char('n') => app.mode = AppMode::Create,                KeyCode::Char('c') => app.copy_selected(),                KeyCode::Char('x') => app.cut_selected(),                KeyCode::Char('p') => app.paste(),                KeyCode::Char('s') => app.save_file(),                KeyCode::Char('o') => app.open_file(),                KeyCode::Char('h') => app.toggle_hidden_files(),                KeyCode::Char('v') => app.view_file(),                KeyCode::Char('e') => app.edit_file(),                KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                app.mode = AppMode::Find
                            }
                            KeyCode::Char('f') => app.mode = AppMode::Filter,                KeyCode::Char('r') => {
                                if key.modifiers.contains(KeyModifiers::CONTROL) {
                                    app.mode = AppMode::Replace;
                                } else {
                                    app.mode = AppMode::Rename;
                                }
                            }
                            KeyCode::Char('+') => app.mode = AppMode::CreateDirectory,                KeyCode::Delete => app.delete_selected(),                KeyCode::Char('m') => app.mode = AppMode::Move,                KeyCode::Right => app.panel_focus = PanelFocus::Actions,                KeyCode::Tab => app.panel_focus = PanelFocus::TopBarButtons,                KeyCode::Left => {}
                            KeyCode::Backspace => app.go_to_parent_directory(),                _ => {}
                        },            PanelFocus::Actions => match key.code {
                            KeyCode::Up => {
                                if app.selected_action > 0 {
                                    app.selected_action -= 1;
                                    app.action_list_state.select(Some(app.selected_action));
                                }
                            }
                            KeyCode::Down => {
                                if app.selected_action < 13 {
                                    app.selected_action += 1;
                                    app.action_list_state.select(Some(app.selected_action));
                                }
                            }
                            KeyCode::Left => app.panel_focus = PanelFocus::Files,                KeyCode::Enter => {
                                match app.selected_action {
                                    0 => app.cut_selected(),                        1 => app.copy_selected(),                        2 => app.paste(),                        3 => app.delete_selected(),                        4 => app.mode = AppMode::Rename,                        5 => app.mode = AppMode::Create,                        6 => app.mode = AppMode::CreateDirectory,                        7 => app.mode = AppMode::Move,                        8 => app.view_file(),                        9 => app.edit_file(),                        10 => app.mode = AppMode::Find,                        11 => app.mode = AppMode::Replace,                        12 => app.toggle_hidden_files(),                        13 => return Ok(()),                        _ => {}
                                }
                                app.mode = AppMode::Normal;
                                app.panel_focus = PanelFocus::Files;
                            }
                            KeyCode::Esc => {
                                app.mode = AppMode::Normal;
                                app.panel_focus = PanelFocus::Files;
                            }
                            _ => {}
                        },            PanelFocus::TopBarButtons => match key.code {
                            KeyCode::Left => {
                                app.top_bar_focus = match app.top_bar_focus {
                                    TopBarFocus::AddressBar => TopBarFocus::UpButton,                        TopBarFocus::PrevButton => TopBarFocus::AddressBar,                        TopBarFocus::NextButton => TopBarFocus::PrevButton,                        TopBarFocus::UpButton => TopBarFocus::NextButton,                    };
                            }
                            KeyCode::Right => {
                                app.top_bar_focus = match app.top_bar_focus {
                                    TopBarFocus::AddressBar => TopBarFocus::PrevButton,                        TopBarFocus::PrevButton => TopBarFocus::NextButton,                        TopBarFocus::NextButton => TopBarFocus::UpButton,                        TopBarFocus::UpButton => TopBarFocus::AddressBar,                    };
                            }
                            KeyCode::Enter => {
                                match app.top_bar_focus {
                                    TopBarFocus::AddressBar => app.mode = AppMode::Editing,                        TopBarFocus::PrevButton => app.go_back(),                        TopBarFocus::NextButton => app.go_forward(),                        TopBarFocus::UpButton => app.go_to_parent_directory(),                    }
                                if app.top_bar_focus != TopBarFocus::AddressBar {
                                    app.panel_focus = PanelFocus::Files;
                                }
                            }
                            KeyCode::Esc | KeyCode::Down => {
                                app.panel_focus = PanelFocus::Files;
                                app.mode = AppMode::Normal;
                            }
                            _ => {}
                        },        },        AppMode::ConfirmDelete => match key.code {
                        KeyCode::Char('y') => app.confirm_delete(),            KeyCode::Char('n') => app.cancel_delete(),            _ => {}
                    },        AppMode::Editing => match key.code {
                        KeyCode::Char(c) => {
                            app.address_input.insert(app.cursor_position, c);
                            app.cursor_position += 1;
                        }
                        KeyCode::Backspace => {
                            if app.cursor_position > 0 {
                                app.cursor_position -= 1;
                                app.address_input.remove(app.cursor_position);
                            }
                        }
                        KeyCode::Enter => {
                            let new_path = PathBuf::from(&app.address_input);
                            if new_path.is_dir() {
                                app.update_path(new_path);
                            }
                            app.mode = AppMode::Normal;
                            app.panel_focus = PanelFocus::Files;
                        }
                        KeyCode::Esc => {
                            app.mode = AppMode::Normal;
                            app.panel_focus = PanelFocus::Files;
                        }
                        _ => {}
                    },        AppMode::Create => match key.code {
                        KeyCode::Char(c) => {
                            app.create_input.push(c);
                        }
                        KeyCode::Backspace => {
                            app.create_input.pop();
                        }
                        KeyCode::Enter => {
                            let new_path = app.path.join(&app.create_input);
                            if new_path.ends_with("/") {
                                fs::create_dir_all(new_path).unwrap();
                            } else {
                                fs::File::create(new_path).unwrap();
                            }
                            app.files = App::get_files(&app.path, app.show_hidden);
                            app.create_input.clear();
                            app.mode = AppMode::Normal;
                        }
                        KeyCode::Esc => {
                            app.create_input.clear();
                            app.mode = AppMode::Normal;
                        }
                        _ => {}
                    },        AppMode::Rename => match key.code {
                        KeyCode::Char(c) => {
                            app.rename_input.push(c);
                        }
                        KeyCode::Backspace => {
                            app.rename_input.pop();
                        }
                        KeyCode::Enter => {
                            let old_path = app.path.join(&app.files[app.selected]);
                            let new_path = app.path.join(&app.rename_input);
                            fs::rename(old_path, new_path).unwrap();
                            app.files = App::get_files(&app.path, app.show_hidden);
                            app.rename_input.clear();
                            app.mode = AppMode::Normal;
                        }
                        KeyCode::Esc => {
                            app.rename_input.clear();
                            app.mode = AppMode::Normal;
                        }
                        _ => {}
                    },        AppMode::Filter => match key.code {
                        KeyCode::Char(c) => {
                            app.filter_input.push(c);
                        }
                        KeyCode::Backspace => {
                            app.filter_input.pop();
                        }
                        KeyCode::Enter => {
                            app.files = App::get_files(&app.path, app.show_hidden);
                            app.files.retain(|f| f.contains(&app.filter_input));
                            app.selected = 0;
                            app.mode = AppMode::Normal;
                        }
                        KeyCode::Esc => {
                            app.filter_input.clear();
                            app.files = App::get_files(&app.path, app.show_hidden);
                            app.mode = AppMode::Normal;
                        }
                        _ => {}
                    },        AppMode::View => match key.code {
                        KeyCode::Esc => app.mode = AppMode::Normal,            _ => {}
                    },        AppMode::Edit => match key.code {
                        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            let selected_file = &app.files[app.selected];
                            let path = app.path.join(selected_file);
                            fs::write(path, &app.edit_content).unwrap();
                            app.edit_content.clear();
                            app.mode = AppMode::Normal;
                        }
                        KeyCode::Char(c) => {
                            app.edit_content.push(c);
                        }
                        KeyCode::Backspace => {
                            app.edit_content.pop();
                        }
                        KeyCode::Enter => {
                            let selected_file = &app.files[app.selected];
                            let path = app.path.join(selected_file);
                            fs::write(path, &app.edit_content).unwrap();
                            app.edit_content.clear();
                            app.mode = AppMode::Normal;
                        }
                        KeyCode::Esc => {
                            app.edit_content.clear();
                            app.mode = AppMode::Normal;
                        }
                        _ => {}
                    },        AppMode::CreateDirectory => match key.code {
                        KeyCode::Char(c) => {
                            app.create_directory_input.push(c);
                        }
                        KeyCode::Backspace => {
                            app.create_directory_input.pop();
                        }
                        KeyCode::Enter => {
                            let new_path = app.path.join(&app.create_directory_input);
                            fs::create_dir_all(new_path).unwrap();
                            app.files = App::get_files(&app.path, app.show_hidden);
                            app.create_directory_input.clear();
                            app.mode = AppMode::Normal;
                        }
                        KeyCode::Esc => {
                            app.create_directory_input.clear();
                            app.mode = AppMode::Normal;
                        }
                        _ => {}
                    },        AppMode::Move => match key.code {
                        KeyCode::Char(c) => {
                            app.move_input.push(c);
                        }
                        KeyCode::Backspace => {
                            app.move_input.pop();
                        }
                        KeyCode::Enter => {
                            let old_path = app.path.join(&app.files[app.selected]);
                            let new_path = PathBuf::from(&app.move_input);
                            fs::rename(old_path, new_path).unwrap();
                            app.files = App::get_files(&app.path, app.show_hidden);
                            app.move_input.clear();
                            app.mode = AppMode::Normal;
                        }
                        KeyCode::Esc => {
                            app.move_input.clear();
                            app.mode = AppMode::Normal;
                        }
                        _ => {}
                    },        AppMode::Find => match key.code {
                        KeyCode::Char(c) => {
                            app.find_input.push(c);
                        }
                        KeyCode::Backspace => {
                            app.find_input.pop();
                        }
                        KeyCode::Enter => {
                            app.mode = AppMode::Normal;
                        }
                        KeyCode::Esc => {
                            app.find_input.clear();
                            app.mode = AppMode::Normal;
                        }
                        _ => {}
                    },        AppMode::Replace => match key.code {
                        KeyCode::Char(c) => {
                            app.replace_input.push(c);
                        }
                        KeyCode::Backspace => {
                            app.replace_input.pop();
                        }
                        KeyCode::Enter => {
                            app.mode = AppMode::Normal;
                        }
                        KeyCode::Esc => {
                            app.replace_input.clear();
                            app.mode = AppMode::Normal;
                        }
                        _ => {}
                    },    }
            } else if let Event::Mouse(mouse_event) = event::read()? {
                if let event::MouseEventKind::Down(button) = mouse_event.kind {
                    if button == event::MouseButton::Left {
                        let (x, y) = (mouse_event.column, mouse_event.row);
                        if address_bar_rect.contains(ratatui::layout::Position::new(x, y)) {
                            app.panel_focus = PanelFocus::TopBarButtons;
                            app.top_bar_focus = TopBarFocus::AddressBar;
                            app.mode = AppMode::Editing;
                            app.cursor_position =
                                (x - address_bar_rect.x).saturating_sub(1) as usize;
                        } else if prev_button_rect.contains(ratatui::layout::Position::new(x, y)) {
                            app.go_back();
                            app.panel_focus = PanelFocus::Files;
                            app.mode = AppMode::Normal;
                        } else if next_button_rect.contains(ratatui::layout::Position::new(x, y)) {
                            app.go_forward();
                            app.panel_focus = PanelFocus::Files;
                            app.mode = AppMode::Normal;
                        } else if up_button_rect.contains(ratatui::layout::Position::new(x, y)) {
                            app.go_to_parent_directory();
                            app.panel_focus = PanelFocus::Files;
                            app.mode = AppMode::Normal;
                        } else if file_list_rect.contains(ratatui::layout::Position::new(x, y)) {
                            app.panel_focus = PanelFocus::Files;
                            let clicked_index = (y - file_list_rect.y) as usize;
                            if clicked_index < app.files.len() {
                                app.selected = clicked_index;
                            }
                        } else if action_list_rect.contains(ratatui::layout::Position::new(x, y)) {
                            app.panel_focus = PanelFocus::Actions;
                            let clicked_index = (y - action_list_rect.y) as usize;
                            if clicked_index < 14 {
                                app.selected_action = clicked_index;
                            }
                        }
                    }
                }
            }
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    execute!(
        terminal.backend_mut(),EnterAlternateScreen,EnableMouseCapture
    )?;
    let mut app = App::new(env::current_dir().unwrap());
    let res = run_app(&mut terminal, &mut app);
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),LeaveAlternateScreen,DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    if let Err(err) = res {
        println!("{err:?}");
    }
    Ok(())
}

