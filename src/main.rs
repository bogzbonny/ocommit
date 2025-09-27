use {
    crossterm::{
        cursor::{MoveToColumn, MoveToPreviousLine},
        event::{self, Event, KeyCode},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
    },
    ollama_rs::{
        generation::{
            completion::request::GenerationRequest,
            parameters::{FormatType, JsonSchema, JsonStructure},
        },
        models::ModelOptions,
        Ollama,
    },
    serde::{Deserialize, Serialize},
    std::{
        env, fs,
        io::{self, stdout, Write},
        path::PathBuf,
        process::Command,
    },
};

/// Configuration stored at ``$HOME/.config/ocommit.yaml``.
#[derive(Debug, Serialize, Deserialize)]
struct Config {
    /// Required model name for Ollama.
    ollama_model: String,
    /// Ignore these filenames for the diff to send to ollama.
    ignore_files: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            ollama_model: "qwen2.5-coder:3b".to_string(),
            ignore_files: vec!["Cargo.lock".to_string()],
        }
    }
}

/// Load configuration, creating the file with defaults if it does not exist.
fn load_config() -> Config {
    let home = env::var("HOME").expect("HOME not set");
    let cfg_path: PathBuf = PathBuf::from(home)
        .join(".config")
        .join("ocommit")
        .join("config.yaml");
    if !cfg_path.exists() {
        println!(
            "config doesn't exist, creating config file at {}",
            cfg_path.display()
        );
        if let Some(parent) = cfg_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let default = Config::default();
        let yaml = serde_yaml::to_string(&default).expect("Failed to serialize config");
        fs::write(&cfg_path, yaml).expect("Failed to write config file");
        return default;
    }
    let contents = fs::read_to_string(&cfg_path).expect("Failed to read config file");
    serde_yaml::from_str(&contents).expect("Invalid config format")
}

#[allow(dead_code)]
#[derive(JsonSchema, Deserialize, Debug)]
struct Output {
    commit_msg: String,
}

/// Generate a commit message from a git diff using Ollama.
async fn generate_message(diff: &str, cfg: &Config) -> Result<String, Box<dyn std::error::Error>> {
    let model = cfg.ollama_model.clone();
    let prompt = format!(
        "Respond in JSON. Generate a concise git commit message for the provided diff:\n{}",
        diff
    );

    let format = FormatType::StructuredJson(Box::new(JsonStructure::new::<Output>()));
    //dbg!(&format);
    let res = Ollama::default()
        .generate(
            GenerationRequest::new(model, prompt)
                .format(format)
                .options(ModelOptions::default().temperature(1.0)),
        )
        .await?;
    let resp: Output = serde_json::from_str(&res.response)?;
    Ok(resp.commit_msg.trim().to_string())
}

/// Obtain the current git diff against HEAD.
/// if there are 1000 > lines changed lines, only include the file names
fn git_diff(cfg: &Config) -> String {
    let mut args = vec!["diff", "HEAD", "--", ":"];
    for file in &cfg.ignore_files {
        args.push("!");
        args.push(file);
    }
    let out = Command::new("git")
        .args(args)
        .output()
        .expect("Failed to execute git diff");
    let mut out = String::from_utf8_lossy(&out.stdout).to_string();
    // 60_000 ~= 1000 LOC at 60 chars per line
    if out.len() > 60_000 {
        let mut args = vec!["diff", "HEAD", "--name-status", "--", ":"];
        for file in &cfg.ignore_files {
            args.push("!");
            args.push(file);
        }
        let out_ = Command::new("git")
            .args(args)
            .output()
            .expect("Failed to execute git diff");
        out = String::from_utf8_lossy(&out_.stdout).to_string();
    }
    out
}

/// Run `git status` and return its stdout.
fn run_git_status() {
    let out = Command::new("git")
        .args(["-c", "color.status=always", "status"])
        .output()
        .expect("Failed to execute git status");
    io::stdout().write_all(&out.stdout).unwrap();
}

fn run_git_add() {
    let _ = Command::new("git")
        .args(["-c", "color.status=always", "add", "-A"])
        .status();
}

fn run_git_commit(msg: &str) {
    let _ = Command::new("git")
        .args(["-c", "color.status=always", "commit", "-m", msg])
        .status();
}

pub fn wait_for_single_key() -> KeyCode {
    enable_raw_mode().expect("enable raw mode");
    loop {
        if event::poll(std::time::Duration::from_millis(100)).expect("poll")
            && let Event::Key(key_event) = event::read().expect("read")
        {
            disable_raw_mode().expect("disable raw mode");
            return key_event.code;
        }
    }
}

fn clear_prev_line(lines: usize) {
    let mut stdout = stdout();
    for _ in 0..lines {
        execute!(
            stdout,
            MoveToPreviousLine(1),
            MoveToColumn(0),
            Clear(ClearType::CurrentLine)
        )
        .unwrap();
    }
}

#[tokio::main]
async fn main() {
    // Simple CLI parsing for a dry‑run flag.
    let args: Vec<String> = env::args().collect();
    let dry = args.iter().any(|a| a == "--dry" || a == "-d");
    let cfg = load_config();

    // Display git status and wait for user confirmation.
    run_git_status();
    let diff = git_diff(&cfg);
    if diff.trim().is_empty() {
        eprintln!("No changes to commit.");
        return;
    }

    println!("-------------------------------------------------------");
    println!("Generating commit message... \n\n");
    let msg = loop {
        let msg = match generate_message(&diff, &cfg).await {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Failed to generate commit message: {}", e);
                return;
            }
        };
        clear_prev_line(3);
        println!("Press ENTER to accept, 'r' or TAB to retry, or ESC to cancel.");
        println!("commit message: \n\t{msg}");

        let key = wait_for_single_key();
        match key {
            KeyCode::Enter => break msg,
            KeyCode::Tab | KeyCode::Char('r') => continue,
            _ => return,
        }
    };

    if dry {
        println!("dry run commit message: \n\t{}", msg);
        return;
    }
    run_git_add();
    run_git_commit(&msg);
}
