use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers}, execute, terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use nerd_font_symbols::md;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Clear, Gauge},
};
use std::{
    env, error::Error, fs, io::{self}, path::{Path, PathBuf}, process::Stdio,
};
use trash;
use viuer;
use xdg;
use image::GenericImageView;
use soloud::*;


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
}

struct App<'a> {
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
    music_playing: bool,
    music_paused: bool,
    music_progress: f64,
    current_music_file: Option<PathBuf>,
    sl: &'a mut Soloud,
    active_handle: Option<Handle>,
}

impl<'a> App<'a> {
    fn new(path: PathBuf, sl: &'a mut Soloud) -> Self {
        let normalized_path = Self::normalize_path(&path);
        let files = Self::get_files(&normalized_path, true);

        Self {
            path: normalized_path.clone(),
            files,
            selected: 0,
            mode: AppMode::Normal,
            address_input: normalized_path.to_str().unwrap().to_string(),
            cursor_position: normalized_path.to_str().unwrap().len(),
            create_input: String::new(),
            rename_input: String::new(),
            clipboard: None,
            is_cut: false,
            show_hidden: true,
            filter_input: String::new(),
            file_content: String::new(),
            edit_content: String::new(),
            create_directory_input: String::new(),
            move_input: String::new(),
            find_input: String::new(),
            replace_input: String::new(),
            selected_action: 0,
            panel_focus: PanelFocus::Files,
            action_list_state: ListState::default(),
            music_playing: false,
            music_paused: false,
            music_progress: 0.0,
            current_music_file: None,
            sl,
            active_handle: None,
        }
    }
    fn on_tick(&mut self) {
        if self.music_playing && !self.music_paused {
            if let Some(handle) = self.active_handle {
                let pos = self.sl.stream_position(handle);
                let len = self.sl.stream_time(handle);
                self.music_progress = pos / len;
            }
        }
    }
    fn normalize_path(path: &Path) -> PathBuf {
        if path.starts_with("~") {
            let home = env::var("HOME").unwrap();
            let mut new_path = PathBuf::new();
            new_path.push(home);
            new_path.push(path.strip_prefix("~").unwrap());
            new_path
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
            a.file_name()
                .unwrap()
                .to_ascii_lowercase()
                .cmp(&b.file_name().unwrap().to_ascii_lowercase())
        });
        let mut hidden_dirs = Vec::new();
        let mut normal_dirs = Vec::new();
        let mut hidden_files = Vec::new();
        let mut normal_files = Vec::new();
        for entry_path in all_entries {
            let file_name = entry_path.file_name().unwrap().to_string_lossy().to_string();
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
            self.path = normalized_path;
            self.files = Self::get_files(&self.path, self.show_hidden);
            self.selected = 0;
        } else {
            let xdg_dirs = xdg::BaseDirectories::with_prefix("karu").unwrap();
            let _ = xdg_dirs.find_cache_file(&normalized_path).map(|p| {
                let _ = std::process::Command::new("xdg-open")
                .arg(p)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
            });
        }
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
            let content = fs::read_to_string(path).unwrap_or_else(|_| "Cannot read file".to_string());
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
            self.file_content = fs::read_to_string(path).unwrap_or_else(|_| "Cannot read file".to_string());
            self.mode = AppMode::View;
        }
    }
            fn edit_file(&mut self) {
                let selected_file = &self.files[self.selected];
                let path = self.path.join(selected_file);
                if !path.is_dir() {
                    self.edit_content = fs::read_to_string(path).unwrap_or_else(|_| "Cannot read file".to_string());
                    self.mode = AppMode::Edit;
                }
            }
            fn play_music(&mut self, path: PathBuf) {
                self.stop_music();
                let mut wav = audio::Wav::default();
                wav.load(&path).unwrap();
                let handle = self.sl.play(&wav);
                self.active_handle = Some(handle);
                self.music_playing = true;
                self.music_paused = false;
                self.current_music_file = Some(path);
            }
            fn toggle_play_pause_music(&mut self) {
                if let Some(handle) = self.active_handle {
                    self.sl.set_pause(handle, !self.music_paused);
                    self.music_paused = !self.music_paused;
                    self.music_playing = !self.music_paused;
                }
            }
            fn stop_music(&mut self) {
                if let Some(handle) = self.active_handle {
                    self.sl.stop(handle);
                    self.music_playing = false;
                    self.music_paused = false;
                    self.current_music_file = None;
                    self.active_handle = None;
                }
            }

            fn seek_forward(&mut self) {
                if let Some(handle) = self.active_handle {
                    let pos = self.sl.stream_position(handle);
                    self.sl.seek(handle, pos + 5.0).unwrap();
                }
            }

            fn seek_backward(&mut self) {
                if let Some(handle) = self.active_handle {
                    let pos = self.sl.stream_position(handle);
                    self.sl.seek(handle, pos - 5.0).unwrap();
                }
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
            main_chunks[0].x + app.cursor_position as u16 + 1, main_chunks[0].y + 1,
        ));
    }
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(main_chunks[1]);
    let file_list_width = content_chunks[0].width;
    let file_list = render_file_list(app, file_list_width, &app.panel_focus);
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
        let block = Block::default().title("Confirm Delete").borders(Borders::ALL);
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
            area.x + app.create_input.len() as u16 + 1, area.y + 1,
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
            area.x + app.rename_input.len() as u16 + 1, area.y + 1,
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
            area.x + app.filter_input.len() as u16 + 1, area.y + 1,
        ));
    }
    if let AppMode::View = app.mode {
        let block = Block::default().title("View File").borders(Borders::ALL);
        let area = centered_rect(80, 80, f.area());
        f.render_widget(Clear, area);
        f.render_widget(block, area);
        let p = Paragraph::new(app.file_content.as_str());
        f.render_widget(p, area);
    }
    if let AppMode::Edit = app.mode {
        let block = Block::default().title("Edit File").borders(Borders::ALL);
        let area = centered_rect(80, 80, f.area());
        f.render_widget(Clear, area);
        f.render_widget(block, area);
        let p = Paragraph::new(app.edit_content.as_str());
        f.render_widget(p, area);
        f.set_cursor_position(Position::new(
            area.x + app.edit_content.len() as u16 + 1, area.y + 1,
        ));
    }
    if let AppMode::CreateDirectory = app.mode {
        let block = Block::default().title("Create Directory").borders(Borders::ALL);
        let area = centered_rect(60, 20, f.area());
        f.render_widget(Clear, area);
        f.render_widget(block, area);
        let p = Paragraph::new(app.create_directory_input.as_str());
        f.render_widget(p, area);
        f.set_cursor_position(Position::new(
            area.x + app.create_directory_input.len() as u16 + 1, area.y + 1,
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
            area.x + app.move_input.len() as u16 + 1, area.y + 1,
        ));
    }
    if let AppMode::Find = app.mode {
        let block = Block::default().title("Find").borders(Borders::ALL);
        let area = centered_rect(60, 20, f.area());
        f.render_widget(Clear, area);
        f.render_widget(block, area);
        let p = Paragraph::new(app.find_input.as_str());
        f.render_widget(p, area);
        f.set_cursor_position(Position::new(
            area.x + app.find_input.len() as u16 + 1, area.y + 1,
        ));
    }
    if let AppMode::Replace = app.mode {
        let block = Block::default().title("Replace").borders(Borders::ALL);
        let area = centered_rect(60, 20, f.area());
        f.render_widget(Clear, area);
        f.render_widget(block, area);
        let p = Paragraph::new(app.replace_input.as_str());
        f.render_widget(p, area);
        f.set_cursor_position(Position::new(
            area.x + app.replace_input.len() as u16 + 1, area.y + 1,
        ));
    }
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
            let glyph = if path.is_dir() {
                md::MD_FOLDER_OPEN
            } else {
                md::MD_FILE
            };
            let display_name = if i.len() > 40 {
                format!("{}...", &i[0..37])
            } else {
                i.clone()
            };
            let glyph_str = glyph.trim();
            let display_name_str = display_name.trim();

            let mut item_text = format!("{}  {}", glyph_str, display_name_str);

            if !path.is_dir() {
                if let Ok(metadata) = fs::metadata(&path) {
                    let size = metadata.len();
                    let formatted_size = format_size(size);
                    let current_len = item_text.len();
                    let size_len = formatted_size.len();
                    let padding = (max_width as usize).saturating_sub(current_len + size_len + 4); // 2 for borders, 2 for highlight symbol
                    item_text = format!("{:<width$}{}", item_text, formatted_size, width = current_len + padding);
                }
            }
            ListItem::new(item_text)
        })
        .collect();
    let mut list = List::new(items)
        .block(Block::default().title("Files").borders(Borders::ALL));
    if let PanelFocus::Files = panel_focus {
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

fn render_context_menu(panel_focus: &PanelFocus) -> List {
    const ACTIONS: &[(&str, &str)] = &[
        ("Cut", "x"),
        ("Copy", "c"),
        ("Paste", "p"),
        ("Delete", "d"),
        ("Rename", "r"),
        ("Create", "n"),
        ("Create Directory", "+"),
        ("Move", "m"),
        ("View", "v"),
        ("Edit", "e"),
        ("Find", "Ctrl+f"),
        ("Replace", "Ctrl+r"),
        ("Toggle Hidden", "h"),
        ("Quit", "q"),
    ];
    let items: Vec<ListItem> = ACTIONS
        .iter()
        .map(|(action, shortcut)| {
            ListItem::new(format!("{} ({})", action, shortcut))
        })
        .collect();
    let mut list = List::new(items)
        .block(Block::default().title("Actions").borders(Borders::ALL));
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
    let selected_file = &app.files[app.selected];
    let path = app.path.join(selected_file);

    // Check file size for preview
    if let Ok(metadata) = fs::metadata(&path) {
        const MAX_PREVIEW_SIZE_MB: u64 = 300;
        const MAX_PREVIEW_SIZE_BYTES: u64 = MAX_PREVIEW_SIZE_MB * 1024 * 1024; // 300 MB in bytes

        if metadata.len() > MAX_PREVIEW_SIZE_BYTES {
            let p = Paragraph::new(format!(
                "File is too large for preview ({}). Max size is {} MB.",
                format_size(metadata.len()),
                MAX_PREVIEW_SIZE_MB
            ))
            .block(Block::default().title("Preview").borders(Borders::ALL));
            f.render_widget(p, area);
            return; // Exit the function early
        }
    }

    if is_image(&path) {
        if let Ok(img) = image::open(&path) {
            let (img_width, img_height) = img.dimensions();
            let max_display_width = area.width.saturating_sub(4) as f32;
            let max_display_height = area.height.saturating_sub(2) as f32;
            let width_ratio = max_display_width / img_width as f32;
            let height_ratio = max_display_height / img_height as f32;
            let scale_factor = width_ratio.min(height_ratio);
            let final_width = (img_width as f32 * scale_factor) as u32;
            let final_height = (img_height as f32 * scale_factor) as u32;
            let centered_x = area.x + (area.width.saturating_sub(final_width as u16)) / 2;
            let centered_y = area.y as i16 + (area.height.saturating_sub(final_height as u16)) as i16 / 2;
            let config = viuer::Config { x: centered_x, y: centered_y, width: Some(final_width), height: Some(final_height), ..Default::default() };
            viuer::print_from_file(path, &config).expect("Image printing failed.");
        } else {
            let p = Paragraph::new("Could not load image").block(Block::default().title("Preview").borders(Borders::ALL));
            f.render_widget(p, area);
        }
    
    } else if is_music(&path) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
            .split(area);

        let controls = if app.music_playing {
            "Playing: (Space) Pause | (s) Stop"
        } else {
            "Paused: (Space) Play | (s) Stop"
        };
        let p = Paragraph::new(controls).block(Block::default().title("Music Player").borders(Borders::ALL));
        f.render_widget(p, chunks[0]);

        let gauge = Gauge::default()
            .block(Block::default().title("Progress").borders(Borders::ALL))
            .gauge_style(Style::default().fg(Color::White).bg(Color::Black).add_modifier(Modifier::ITALIC))
            .percent(app.music_progress as u16);
        f.render_widget(gauge, chunks[1]);
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
                    format!("{}...", &line[0..max_width.saturating_sub(3)])
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<String>>()
            .join("\n");
        let p = Paragraph::new(truncated_content).block(Block::default().title("Preview").borders(Borders::ALL)).style(Style::default().bg(Color::Reset));
        f.render_widget(p, area);
    }
}
fn is_image(path: &Path) -> bool {
    let extension = path.extension().and_then(|s| s.to_str());
    if let Some(ext) = extension {
        matches!(ext.to_lowercase().as_str(), "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "tiff" | "webp")
    } else {
        false
    }
}
fn is_music(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext_str = ext.to_ascii_lowercase();
        return matches!(ext_str.to_str(), Some("mp3") | Some("wav") | Some("ogg") | Some("flac"));
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
                Constraint::Percentage((100 - percent_y) / 2), Constraint::Percentage(percent_y), Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2), Constraint::Percentage(percent_x), Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

fn run_app(

    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,

    app: &mut App,

) -> io::Result<()> {

    let mut last_tick = std::time::Instant::now();

    loop {

        terminal.draw(|f| ui(f, app))?;



        let timeout = std::time::Duration::from_millis(250)

            .checked_sub(last_tick.elapsed())

            .unwrap_or_else(|| std::time::Duration::from_secs(0));



        if crossterm::event::poll(timeout)? {

            if let Event::Key(key) = event::read()? {

                match app.mode {

                    AppMode::Normal => {

                        match app.panel_focus {

                            PanelFocus::Files => {

                                match key.code {

                                    KeyCode::Char('q') => return Ok(()),

                                    KeyCode::Down => app.select_next(),

                                    KeyCode::Up => app.select_previous(),

                                    KeyCode::Enter => {

                                        let selected_file = &app.files[app.selected];

                                        let path = app.path.join(selected_file);

                                        if is_music(&path) {

                                            app.play_music(path);

                                        } else {

                                            app.open_selected();

                                        }

                                    },

                                    KeyCode::Char('d') => app.delete_selected(),

                                    KeyCode::Char('/') => app.mode = AppMode::Editing,

                                    KeyCode::Char('n') => app.mode = AppMode::Create,

                                    KeyCode::Char('c') => app.copy_selected(),

                                    KeyCode::Char('x') => app.cut_selected(),

                                    KeyCode::Char('p') => app.paste(),

                                    KeyCode::Char('s') => {

                                        if app.music_playing {

                                            app.stop_music();

                                        } else {

                                            app.save_file();

                                        }

                                    },

                                    KeyCode::Char('o') => app.open_file(),

                                    KeyCode::Char('h') => app.toggle_hidden_files(),

                                    KeyCode::Char('v') => app.view_file(),

                                    KeyCode::Char('e') => app.edit_file(),

                                    KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => app.mode = AppMode::Find,

                                    KeyCode::Char('f') => app.mode = AppMode::Filter,

                                    KeyCode::Char('r') => {

                                        if key.modifiers.contains(KeyModifiers::CONTROL) {

                                            app.mode = AppMode::Replace;

                                        }

                                        else {

                                            app.mode = AppMode::Rename;

                                        }

                                    },

                                    KeyCode::Char('+') => app.mode = AppMode::CreateDirectory,

                                    KeyCode::Delete => app.delete_selected(),

                                    KeyCode::Char('m') => app.mode = AppMode::Move,

                                    KeyCode::Right => {

                                        if app.current_music_file.is_some() {

                                            app.seek_forward();

                                        } else {

                                            app.panel_focus = PanelFocus::Actions;

                                        }

                                    },

                                    KeyCode::Left => {

                                        if app.current_music_file.is_some() {

                                            app.seek_backward();

                                        }

                                    },

                                    KeyCode::Char(' ') => {

                                        if app.current_music_file.is_some() {

                                            app.toggle_play_pause_music();

                                        }

                                    },

                                    _ => {}

                                }

                            },

                            PanelFocus::Actions => {

                                match key.code {

                                    KeyCode::Up => {

                                        if app.selected_action > 0 {

                                            app.selected_action -= 1;

                                            app.action_list_state.select(Some(app.selected_action));

                                        }

                                    },

                                    KeyCode::Down => {

                                        if app.selected_action < 13 { // 14 actions, 0-indexed

                                            app.selected_action += 1;

                                            app.action_list_state.select(Some(app.selected_action));

                                        }

                                    },

                                    KeyCode::Left => app.panel_focus = PanelFocus::Files,

                                    KeyCode::Enter => {

                                        match app.selected_action {

                                            0 => app.cut_selected(),

                                            1 => app.copy_selected(),

                                            2 => app.paste(),

                                            3 => app.delete_selected(),

                                            4 => app.mode = AppMode::Rename,

                                            5 => app.mode = AppMode::Create,

                                            6 => app.mode = AppMode::CreateDirectory,

                                            7 => app.mode = AppMode::Move,

                                            8 => app.view_file(),

                                            9 => app.edit_file(),

                                            10 => app.mode = AppMode::Find,

                                            11 => app.mode = AppMode::Replace,

                                            12 => app.toggle_hidden_files(),

                                            13 => return Ok(()), // Quit

                                            _ => {}

                                        }

                                        app.mode = AppMode::Normal; // Return to normal mode after action

                                        app.panel_focus = PanelFocus::Files; // Return focus to files panel

                                    },

                                    KeyCode::Esc => {

                                        app.mode = AppMode::Normal;

                                        app.panel_focus = PanelFocus::Files; // Return focus to files panel

                                    },

                                    _ => {}

                                }

                            },

                        }

                    },

                    AppMode::ConfirmDelete => {

                        match key.code {

                            KeyCode::Char('y') => app.confirm_delete(),

                            KeyCode::Char('n') => app.cancel_delete(),

                            _ => {}

                        }

                    },

                    AppMode::Editing => {

                        match key.code {

                            KeyCode::Char(c) => {

                                app.address_input.insert(app.cursor_position, c);

                                app.cursor_position += 1;

                            },

                            KeyCode::Backspace => {

                                if app.cursor_position > 0 {

                                    app.cursor_position -= 1;

                                    app.address_input.remove(app.cursor_position);

                                }

                            },

                            KeyCode::Enter => {

                                let new_path = PathBuf::from(&app.address_input);

                                if new_path.is_dir() {

                                    app.path = new_path;

                                    app.files = App::get_files(&app.path, app.show_hidden);

                                    app.selected = 0;

                                }

                                app.mode = AppMode::Normal;

                            },

                            KeyCode::Esc => app.mode = AppMode::Normal,

                            _ => {}

                        }

                    },

                    AppMode::Create => {

                        match key.code {

                            KeyCode::Char(c) => {

                                app.create_input.push(c);

                            },

                            KeyCode::Backspace => {

                                app.create_input.pop();

                            },

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

                            },

                            KeyCode::Esc => {

                                app.create_input.clear();

                                app.mode = AppMode::Normal;

                            },

                            _ => {}

                        }

                    },

                    AppMode::Rename => {

                        match key.code {

                            KeyCode::Char(c) => {

                                app.rename_input.push(c);

                            },

                            KeyCode::Backspace => {

                                app.rename_input.pop();

                            },

                            KeyCode::Enter => {

                                let old_path = app.path.join(&app.files[app.selected]);

                                let new_path = app.path.join(&app.rename_input);

                                fs::rename(old_path, new_path).unwrap();

                                app.files = App::get_files(&app.path, app.show_hidden);

                                app.rename_input.clear();

                                app.mode = AppMode::Normal;

                            },

                            KeyCode::Esc => {

                                app.rename_input.clear();

                                app.mode = AppMode::Normal;

                            },

                            _ => {}

                        }

                    },

                    AppMode::Filter => {

                        match key.code {

                            KeyCode::Char(c) => {

                                app.filter_input.push(c);

                            },

                            KeyCode::Backspace => {

                                app.filter_input.pop();

                            },

                            KeyCode::Enter => {

                                app.files = App::get_files(&app.path, app.show_hidden);

                                app.files.retain(|f| f.contains(&app.filter_input));

                                app.selected = 0;

                                app.mode = AppMode::Normal;

                            },

                            KeyCode::Esc => {

                                app.filter_input.clear();

                                app.files = App::get_files(&app.path, app.show_hidden);

                                app.mode = AppMode::Normal;

                            },

                            _ => {}

                        }

                    },

                    AppMode::View => {

                        match key.code {

                            KeyCode::Esc => app.mode = AppMode::Normal,

                            _ => {}

                        }

                    },

                    AppMode::Edit => {

                        match key.code {

                            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {

                                let selected_file = &app.files[app.selected];

                                let path = app.path.join(selected_file);

                                fs::write(path, &app.edit_content).unwrap();

                                app.edit_content.clear();

                                app.mode = AppMode::Normal;

                            },

                            KeyCode::Char(c) => {

                                app.edit_content.push(c);

                            },

                            KeyCode::Backspace => {

                                app.edit_content.pop();

                            },

                            KeyCode::Enter => {

                                let selected_file = &app.files[app.selected];

                                let path = app.path.join(selected_file);

                                fs::write(path, &app.edit_content).unwrap();

                                app.edit_content.clear();

                                app.mode = AppMode::Normal;

                            },

                            KeyCode::Esc => {

                                app.edit_content.clear();

                                app.mode = AppMode::Normal;

                            },

                            _ => {}

                        }

                    },

                    AppMode::CreateDirectory => {

                        match key.code {

                            KeyCode::Char(c) => {

                                app.create_directory_input.push(c);

                            },

                            KeyCode::Backspace => {

                                app.create_directory_input.pop();

                            },

                            KeyCode::Enter => {

                                let new_path = app.path.join(&app.create_directory_input);

                                fs::create_dir_all(new_path).unwrap();

                                app.files = App::get_files(&app.path, app.show_hidden);

                                app.create_directory_input.clear();

                                app.mode = AppMode::Normal;

                            },

                            KeyCode::Esc => {

                                app.create_directory_input.clear();

                                app.mode = AppMode::Normal;

                            },

                            _ => {}

                        }

                    },

                    AppMode::Move => {

                        match key.code {

                            KeyCode::Char(c) => {

                                app.move_input.push(c);

                            },

                            KeyCode::Backspace => {

                                app.move_input.pop();

                            },

                            KeyCode::Enter => {

                                let old_path = app.path.join(&app.files[app.selected]);

                                let new_path = PathBuf::from(&app.move_input);

                                fs::rename(old_path, new_path).unwrap();

                                app.files = App::get_files(&app.path, app.show_hidden);

                                app.move_input.clear();

                                app.mode = AppMode::Normal;

                            },

                            KeyCode::Esc => {

                                app.move_input.clear();

                                app.mode = AppMode::Normal;

                            },

                            _ => {}

                        }

                    },

                    AppMode::Find => {

                        match key.code {

                            KeyCode::Char(c) => {

                                app.find_input.push(c);

                            },

                            KeyCode::Backspace => {

                                app.find_input.pop();

                            },

                            KeyCode::Enter => {

                                app.mode = AppMode::Normal;

                            },

                            KeyCode::Esc => {

                                app.find_input.clear();

                                app.mode = AppMode::Normal;

                            },

                            _ => {}

                        }

                    },

                    AppMode::Replace => {

                        match key.code {

                            KeyCode::Char(c) => {

                                app.replace_input.push(c);

                            },

                            KeyCode::Backspace => {

                                app.replace_input.pop();

                            },

                            KeyCode::Enter => {

                                app.mode = AppMode::Normal;

                            },

                            KeyCode::Esc => {

                                app.replace_input.clear();

                                app.mode = AppMode::Normal;

                            },

                            _ => {}

                        }

                    },

                }

            }

        }

        if last_tick.elapsed() >= std::time::Duration::from_millis(250) {

            app.on_tick();

            last_tick = std::time::Instant::now();

        }

    }

}

fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut sl = Soloud::default().unwrap();
    let mut app = App::new(env::current_dir().unwrap(), &mut sl);
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
