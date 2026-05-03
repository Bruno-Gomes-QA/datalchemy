use std::path::{Path, PathBuf};

use chrono::Local;

use crate::CliError;
use crate::tui::secrets::load_env_file;
use crate::tui::utils::append_line;
use crate::workspace::{
    ApprovalPolicy, LlmModels, LlmProvider, PrivacyMode, WorkspaceMode, WorkspacePaths,
    WorkspaceSettings, WriteIntent, load_or_create_llm_models, load_or_create_profiles,
    load_or_create_settings, write_json_atomic,
};

pub const MAX_MESSAGES: usize = 2000;

#[derive(Debug, Clone)]
pub enum InputMode {
    Command,
    Approval {
        intent: WriteIntent,
        command: String,
    },
}

#[derive(Debug, Clone)]
pub enum UiState {
    Normal,
    Setup(SetupStep),
}

#[derive(Debug, Clone)]
pub enum SetupStep {
    Welcome,
    ConfirmWorkspace,
    ConfirmReset,
    ProfileName,
    ConnectionString,
    DbSession,
    DbChange,
    SelectSchema,
    Introspecting,
    Prompt(PromptContext),
}

/// Generic multi-step interactive prompt for commands that need parameters.
#[derive(Debug, Clone)]
pub struct PromptContext {
    pub command: String,
    pub prompts: Vec<String>,
    pub collected: Vec<String>,
}

impl PromptContext {
    pub fn new(command: &str, prompts: Vec<&str>) -> Self {
        Self {
            command: command.to_string(),
            prompts: prompts.into_iter().map(|s| s.to_string()).collect(),
            collected: Vec::new(),
        }
    }

    pub fn current_prompt(&self) -> Option<&str> {
        self.prompts.get(self.collected.len()).map(|s| s.as_str())
    }

    pub fn is_complete(&self) -> bool {
        self.collected.len() >= self.prompts.len()
    }

    pub fn push(&mut self, value: String) {
        self.collected.push(value);
    }
}

#[derive(Debug, Clone)]
pub struct PaletteEntry {
    pub command: String,
    pub description: String,
}

pub enum AppEvent {
    Log(String),
    SchemasLoaded(Result<Vec<String>, String>),
    IntrospectionDone(Result<(), String>),
    AsyncTaskDone(String),
}

/// Tracks a long-running async operation for progress display.
#[derive(Debug, Clone)]
pub struct ActiveTask {
    pub label: String,
    pub started_at: String,
}

pub struct App {
    pub runtime: tokio::runtime::Handle,
    pub tx: tokio::sync::mpsc::UnboundedSender<AppEvent>,
    pub paths: WorkspacePaths,
    pub settings: WorkspaceSettings,
    pub profiles: crate::workspace::ProfilesConfig,
    pub llm_models: LlmModels,
    pub input: String,
    pub cursor_pos: usize,
    pub messages: Vec<String>,
    pub mode: InputMode,
    pub should_quit: bool,
    pub session_conn: Option<String>,
    pub last_out_id: Option<String>,
    pub ui_state: UiState,
    pub setup_profile_name: Option<String>,
    pub scroll_offset: u16,
    pub palette_select: usize,
    pub spinner_idx: usize,
    pub available_schemas: Vec<String>,
    pub schema_picker_idx: usize,
    pub active_task: Option<ActiveTask>,
}

impl App {
    pub fn new(
        runtime: tokio::runtime::Handle,
        workspace_root: PathBuf,
        tx: tokio::sync::mpsc::UnboundedSender<AppEvent>,
    ) -> Result<Self, CliError> {
        let paths = WorkspacePaths::new(workspace_root);
        let mut settings = WorkspaceSettings::default();
        let mut profiles = crate::workspace::ProfilesConfig::default();
        let mut llm_models = LlmModels::default();
        let mut needs_setup = true;

        if paths.root.exists() {
            settings = load_or_create_settings(&paths)?;
            profiles = load_or_create_profiles(&paths)?;
            llm_models = load_or_create_llm_models(&paths)?;
            needs_setup = settings.active_profile.is_none();
        }

        let ui_state = if needs_setup {
            UiState::Setup(SetupStep::Welcome)
        } else {
            UiState::Normal
        };

        Ok(Self {
            runtime,
            tx,
            paths,
            settings,
            profiles,
            llm_models,
            input: String::new(),
            cursor_pos: 0,
            messages: Vec::new(),
            mode: InputMode::Command,
            should_quit: false,
            session_conn: None,
            last_out_id: None,
            ui_state,
            setup_profile_name: None,
            scroll_offset: 0,
            palette_select: 0,
            spinner_idx: 0,
            available_schemas: Vec::new(),
            schema_picker_idx: 0,
            active_task: None,
        })
    }

    // -- input helpers (cursor-aware) --

    pub fn input_insert_char(&mut self, ch: char) {
        self.input.insert(self.cursor_pos, ch);
        self.cursor_pos += ch.len_utf8();
    }

    pub fn input_delete_back(&mut self) {
        if self.cursor_pos > 0 {
            let prev = self.input[..self.cursor_pos]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.input.drain(prev..self.cursor_pos);
            self.cursor_pos = prev;
        }
    }

    pub fn input_delete_forward(&mut self) {
        if self.cursor_pos < self.input.len() {
            let next = self.input[self.cursor_pos..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor_pos + i)
                .unwrap_or(self.input.len());
            self.input.drain(self.cursor_pos..next);
        }
    }

    pub fn input_move_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos = self.input[..self.cursor_pos]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    pub fn input_move_right(&mut self) {
        if self.cursor_pos < self.input.len() {
            self.cursor_pos = self.input[self.cursor_pos..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor_pos + i)
                .unwrap_or(self.input.len());
        }
    }

    pub fn input_move_home(&mut self) {
        self.cursor_pos = 0;
    }

    pub fn input_move_end(&mut self) {
        self.cursor_pos = self.input.len();
    }

    pub fn input_delete_word_back(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }
        let before = &self.input[..self.cursor_pos];
        let trimmed = before.trim_end();
        let new_pos = trimmed
            .rfind(|c: char| c.is_whitespace() || c == '/')
            .map(|i| i + 1)
            .unwrap_or(0);
        self.input.drain(new_pos..self.cursor_pos);
        self.cursor_pos = new_pos;
    }

    pub fn input_clear(&mut self) {
        self.input.clear();
        self.cursor_pos = 0;
    }

    pub fn input_set(&mut self, value: String) {
        self.cursor_pos = value.len();
        self.input = value;
    }

    pub fn input_take(&mut self) -> String {
        self.cursor_pos = 0;
        std::mem::take(&mut self.input)
    }

    // -- message helpers --

    pub fn active_profile_redacted(&self) -> Option<String> {
        self.settings
            .active_profile
            .as_ref()
            .and_then(|name| self.profiles.profiles.get(name))
            .map(|p| p.redacted.clone())
    }

    pub fn iter_runs(&self) -> impl Iterator<Item = String> {
        crate::tui::utils::list_dirs(&self.paths.runs_dir)
            .unwrap_or_default()
            .into_iter()
    }

    pub fn iter_plans(&self) -> impl Iterator<Item = String> {
        crate::tui::utils::list_dirs(&self.paths.plans_dir)
            .unwrap_or_default()
            .into_iter()
    }

    fn timestamp() -> String {
        Local::now().format("%H:%M:%S").to_string()
    }

    pub fn push_message(&mut self, message: impl Into<String>) {
        let line = message.into();
        let stamped = if line.is_empty() {
            String::new()
        } else {
            format!("[{}] {}", Self::timestamp(), line)
        };
        self.messages.push(stamped.clone());
        if self.paths.cli_log_path().exists() {
            let _ = append_line(&self.paths.cli_log_path(), &stamped);
        }
        if self.messages.len() > MAX_MESSAGES {
            let overflow = self.messages.len() - MAX_MESSAGES;
            self.messages.drain(0..overflow);
        }
    }

    pub fn push_raw(&mut self, message: impl Into<String>) {
        let line = message.into();
        self.messages.push(line.clone());
        if self.paths.cli_log_path().exists() {
            let _ = append_line(&self.paths.cli_log_path(), &line);
        }
        if self.messages.len() > MAX_MESSAGES {
            let overflow = self.messages.len() - MAX_MESSAGES;
            self.messages.drain(0..overflow);
        }
    }

    pub fn record_command(&mut self, command: &str) {
        if !self.messages.is_empty() {
            self.push_raw("");
        }
        self.push_raw(format!("► {}", command));
    }

    pub fn is_in_setup(&self) -> bool {
        matches!(self.ui_state, UiState::Setup(_))
    }

    pub fn show_header(&self) -> bool {
        matches!(self.ui_state, UiState::Normal)
    }

    pub fn profile_display(&self) -> String {
        match &self.settings.active_profile {
            None => "none".to_string(),
            Some(name) => {
                if let Some(profile) = self.profiles.profiles.get(name) {
                    match self.settings.privacy {
                        PrivacyMode::Paranoid => name.clone(),
                        PrivacyMode::Normal => format!("{name} ({})", profile.redacted),
                    }
                } else {
                    name.clone()
                }
            }
        }
    }

    pub fn mode_display(&self) -> String {
        match self.settings.mode {
            WorkspaceMode::ReadonlyCsv => "readonly_csv",
            WorkspaceMode::Insert => "insert",
            WorkspaceMode::Explore => "explore",
        }
        .to_string()
    }

    pub fn llm_display(&self) -> String {
        if !self.settings.llm_enabled {
            return "OFF".to_string();
        }
        let provider = match self.settings.llm_provider {
            LlmProvider::Gemini => "gemini",
            LlmProvider::Off => "off",
        };
        let model = self
            .settings
            .llm_model
            .clone()
            .unwrap_or_else(|| "default".to_string());
        format!("{provider}/{model}")
    }

    pub fn write_profile_config(&self, dir: &Path) -> Result<(), CliError> {
        let Some(profile_name) = &self.settings.active_profile else {
            return Ok(());
        };
        let Some(profile) = self.profiles.profiles.get(profile_name) else {
            return Ok(());
        };
        write_json_atomic(&dir.join("config.redacted.json"), profile)?;
        Ok(())
    }

    pub fn requires_approval(&self) -> bool {
        matches!(self.settings.approval_policy, ApprovalPolicy::AskEachTime)
    }

    pub fn request_approval(&mut self, intent: WriteIntent, command: &str) -> Result<(), CliError> {
        self.mode = InputMode::Approval {
            intent,
            command: command.to_string(),
        };
        Ok(())
    }

    pub fn resolve_connection_string(&self) -> Result<String, String> {
        if let Some(conn) = &self.session_conn {
            return Ok(conn.clone());
        }
        if let Ok(conn) = std::env::var("DATABASE_URL") {
            return Ok(conn);
        }
        let env_path = PathBuf::from(".env");
        if env_path.exists() {
            if let Ok(values) = load_env_file(&env_path) {
                if let Some(conn) = values.get("DATABASE_URL") {
                    return Ok(conn.clone());
                }
            }
        }
        Err("missing connection string. use /db session or set DATABASE_URL (or .env).".to_string())
    }

    pub fn start_task(&mut self, label: impl Into<String>) {
        self.active_task = Some(ActiveTask {
            label: label.into(),
            started_at: Self::timestamp(),
        });
    }

    pub fn finish_task(&mut self) {
        self.active_task = None;
    }
}
