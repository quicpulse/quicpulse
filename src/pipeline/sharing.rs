//! Workflow sharing and collaboration
//!
//! Provides functionality to list, pull, push, and share workflows
//! with remote registries like GitHub Gists.

use crate::cli::Args;
use crate::context::Environment;
use crate::errors::QuicpulseError;
use crate::status::ExitStatus;
use std::path::{Path, PathBuf};

/// Default workflow directory
const WORKFLOWS_DIR: &str = ".quicpulse/workflows";

/// Default registry (GitHub Gists API)
const DEFAULT_REGISTRY: &str = "https://api.github.com/gists";

/// Workflow metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkflowMeta {
    pub name: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub source: WorkflowSource,
    pub version: Option<String>,
    pub author: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Workflow source location
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum WorkflowSource {
    Local { path: PathBuf },
    Remote { url: String, id: String },
    GitHub { owner: String, repo: String, path: String },
    Gist { id: String, filename: String },
}

/// Handle workflow sharing commands
pub async fn handle_workflow_commands(args: &Args, env: &Environment) -> Result<Option<ExitStatus>, QuicpulseError> {
    // Handle --workflow-list
    if args.workflow_list {
        list_workflows(args, env).await?;
        return Ok(Some(ExitStatus::Success));
    }

    // Handle --workflow-search
    if let Some(ref query) = args.workflow_search {
        search_workflows(query, args, env).await?;
        return Ok(Some(ExitStatus::Success));
    }

    // Handle --workflow-pull
    if let Some(ref source) = args.workflow_pull {
        pull_workflow(source, args, env).await?;
        return Ok(Some(ExitStatus::Success));
    }

    // Handle --workflow-push
    if let Some(ref path) = args.workflow_push {
        push_workflow(path, args, env).await?;
        return Ok(Some(ExitStatus::Success));
    }

    Ok(None)
}

/// List local and optionally remote workflows
async fn list_workflows(args: &Args, env: &Environment) -> Result<(), QuicpulseError> {
    println!("\x1b[1m=== Local Workflows ===\x1b[0m\n");

    // Find local workflows
    let workflows_dir = get_workflows_dir(env);
    if workflows_dir.exists() {
        let mut found = false;
        for entry in std::fs::read_dir(&workflows_dir).map_err(QuicpulseError::Io)? {
            let entry = entry.map_err(QuicpulseError::Io)?;
            let path = entry.path();
            if is_workflow_file(&path) {
                found = true;
                let name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");

                // Try to read workflow metadata
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let desc = extract_workflow_description(&content);
                    if let Some(d) = desc {
                        println!("  \x1b[36m{}\x1b[0m", name);
                        println!("    {}", d);
                    } else {
                        println!("  \x1b[36m{}\x1b[0m", name);
                    }
                } else {
                    println!("  \x1b[36m{}\x1b[0m", name);
                }
            }
        }

        if !found {
            println!("  (no workflows found)");
        }
    } else {
        println!("  (workflows directory not found: {})", workflows_dir.display());
    }

    // Check current directory too
    println!("\n\x1b[1m=== Current Directory ===\x1b[0m\n");
    let current_dir = std::env::current_dir().unwrap_or_default();
    let mut found_local = false;

    if let Ok(entries) = std::fs::read_dir(&current_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if is_workflow_file(&path) {
                found_local = true;
                let name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");
                println!("  \x1b[36m{}\x1b[0m", name);
            }
        }
    }

    if !found_local {
        println!("  (no workflow files found)");
    }

    if args.verbose > 0 {
        println!("\n\x1b[90mUse --workflow-pull <url> to download workflows\x1b[0m");
        println!("\x1b[90mUse --workflow-search <query> to search remote workflows\x1b[0m");
    }

    Ok(())
}

/// Search for workflows in remote registries
async fn search_workflows(query: &str, _args: &Args, _env: &Environment) -> Result<(), QuicpulseError> {
    println!("\x1b[1mSearching for: {}\x1b[0m\n", query);

    // Search GitHub for workflow files
    let search_url = format!(
        "https://api.github.com/search/code?q={}+extension:yaml+filename:quicpulse+in:path",
        urlencoding::encode(query)
    );

    println!("\x1b[90mSearching GitHub for QuicPulse workflows...\x1b[0m\n");

    match fetch_json(&search_url).await {
        Ok(json) => {
            if let Some(items) = json.get("items").and_then(|v| v.as_array()) {
                if items.is_empty() {
                    println!("No workflows found matching '{}'", query);
                    println!("\n\x1b[90mTip: Try searching with different keywords or check:\x1b[0m");
                    println!("\x1b[90m  https://github.com/topics/quicpulse-workflow\x1b[0m");
                } else {
                    println!("Found {} workflow(s):\n", items.len());
                    for item in items.iter().take(10) {
                        let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
                        let repo = item.get("repository")
                            .and_then(|r| r.get("full_name"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        let html_url = item.get("html_url").and_then(|v| v.as_str()).unwrap_or("");

                        println!("  \x1b[36m{}\x1b[0m", name);
                        println!("    Repo: {}", repo);
                        println!("    URL: {}", html_url);
                        println!();
                    }
                }
            } else {
                println!("No results found");
            }
        }
        Err(e) => {
            eprintln!("Search failed: {}", e);
            println!("\n\x1b[90mTip: You can manually browse workflows at:\x1b[0m");
            println!("\x1b[90m  https://github.com/topics/quicpulse-workflow\x1b[0m");
        }
    }

    Ok(())
}

/// Pull a workflow from a remote source
async fn pull_workflow(source: &str, args: &Args, env: &Environment) -> Result<(), QuicpulseError> {
    println!("Pulling workflow from: {}\n", source);

    let (content, filename) = if source.starts_with("http://") || source.starts_with("https://") {
        // Direct URL
        let content = fetch_text(source).await?;
        let filename = source.split('/').last().unwrap_or("workflow.yaml").to_string();
        (content, filename)
    } else if source.contains('/') {
        // GitHub shorthand: owner/repo/path or owner/repo
        let parts: Vec<&str> = source.split('/').collect();
        if parts.len() >= 2 {
            let owner = parts[0];
            let repo = parts[1];
            let path = if parts.len() > 2 {
                parts[2..].join("/")
            } else {
                "quicpulse.yaml".to_string()
            };

            let url = format!(
                "https://raw.githubusercontent.com/{}/{}/main/{}",
                owner, repo, path
            );

            let content = fetch_text(&url).await?;
            let filename = path.split('/').last().unwrap_or("workflow.yaml").to_string();
            (content, filename)
        } else {
            return Err(QuicpulseError::Argument(
                "Invalid source format. Use: URL, owner/repo, or owner/repo/path".to_string()
            ));
        }
    } else {
        // Assume it's a gist ID
        let url = format!("https://api.github.com/gists/{}", source);
        let json = fetch_json(&url).await?;

        if let Some(files) = json.get("files").and_then(|f| f.as_object()) {
            if let Some((filename, file_obj)) = files.iter().next() {
                let content = file_obj.get("content")
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string();
                (content, filename.clone())
            } else {
                return Err(QuicpulseError::Argument("Gist has no files".to_string()));
            }
        } else {
            return Err(QuicpulseError::Argument("Invalid gist format".to_string()));
        }
    };

    // Save to workflows directory
    let workflows_dir = get_workflows_dir(env);
    std::fs::create_dir_all(&workflows_dir).map_err(QuicpulseError::Io)?;

    let target_path = workflows_dir.join(&filename);
    std::fs::write(&target_path, &content).map_err(QuicpulseError::Io)?;

    println!("\x1b[32mSuccess!\x1b[0m Workflow saved to: {}", target_path.display());

    if args.verbose > 0 {
        // Try to parse and show info
        if let Ok(yaml) = serde_yaml::from_str::<serde_json::Value>(&content) {
            if let Some(name) = yaml.get("name").and_then(|v| v.as_str()) {
                println!("  Name: {}", name);
            }
            if let Some(desc) = yaml.get("description").and_then(|v| v.as_str()) {
                println!("  Description: {}", desc);
            }
        }
    }

    println!("\n\x1b[90mRun with: quicpulse --run {}\x1b[0m", target_path.display());

    Ok(())
}

/// Push a workflow to a remote registry (GitHub Gist)
async fn push_workflow(path: &Path, args: &Args, _env: &Environment) -> Result<(), QuicpulseError> {
    if !path.exists() {
        return Err(QuicpulseError::Argument(format!(
            "Workflow file not found: {}", path.display()
        )));
    }

    let content = std::fs::read_to_string(path).map_err(QuicpulseError::Io)?;
    let filename = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("workflow.yaml");

    // Check for GitHub token
    let token = std::env::var("GITHUB_TOKEN")
        .or_else(|_| std::env::var("GH_TOKEN"))
        .map_err(|_| QuicpulseError::Argument(
            "GitHub token required for pushing. Set GITHUB_TOKEN or GH_TOKEN environment variable.".to_string()
        ))?;

    let extracted_desc = extract_workflow_description(&content);
    let description = args.workflow_description.as_deref()
        .or_else(|| extracted_desc.as_deref())
        .unwrap_or("QuicPulse workflow");

    let is_public = args.workflow_public;

    // Create gist
    let gist_data = serde_json::json!({
        "description": description,
        "public": is_public,
        "files": {
            filename: {
                "content": content
            }
        }
    });

    println!("Creating gist...");

    let registry = args.workflow_registry.as_deref().unwrap_or(DEFAULT_REGISTRY);

    let client = reqwest::Client::new();
    let resp = client.post(registry)
        .header("Authorization", format!("token {}", token))
        .header("User-Agent", "QuicPulse")
        .header("Accept", "application/vnd.github.v3+json")
        .json(&gist_data)
        .send()
        .await
        .map_err(QuicpulseError::Request)?;

    if resp.status().is_success() {
        let json: serde_json::Value = resp.json().await.map_err(QuicpulseError::Request)?;
        let html_url = json.get("html_url").and_then(|v| v.as_str()).unwrap_or("");
        let id = json.get("id").and_then(|v| v.as_str()).unwrap_or("");

        println!("\n\x1b[32mSuccess!\x1b[0m Workflow published.");
        println!("  URL: {}", html_url);
        println!("  ID: {}", id);
        println!("  Visibility: {}", if is_public { "public" } else { "private" });

        println!("\n\x1b[90mOthers can pull with: quicpulse --workflow-pull {}\x1b[0m", id);
    } else {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(QuicpulseError::Argument(format!(
            "Failed to create gist: {} - {}", status, text
        )));
    }

    Ok(())
}

/// Get the workflows directory
fn get_workflows_dir(_env: &Environment) -> PathBuf {
    if let Some(home) = dirs::home_dir() {
        home.join(WORKFLOWS_DIR)
    } else {
        PathBuf::from(WORKFLOWS_DIR)
    }
}

/// Check if a file is a workflow file
fn is_workflow_file(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext = ext.to_str().unwrap_or("");
        if ext == "yaml" || ext == "yml" || ext == "toml" {
            // Try to check if it contains workflow content
            if let Ok(content) = std::fs::read_to_string(path) {
                return content.contains("steps:") || content.contains("[steps]");
            }
        }
    }
    false
}

/// Extract workflow description from content
fn extract_workflow_description(content: &str) -> Option<String> {
    // Try YAML
    if let Ok(yaml) = serde_yaml::from_str::<serde_json::Value>(content) {
        if let Some(desc) = yaml.get("description").and_then(|v| v.as_str()) {
            return Some(desc.to_string());
        }
    }

    // Try TOML
    if let Ok(toml) = toml::from_str::<serde_json::Value>(content) {
        if let Some(desc) = toml.get("description").and_then(|v| v.as_str()) {
            return Some(desc.to_string());
        }
    }

    None
}

/// Fetch JSON from URL
async fn fetch_json(url: &str) -> Result<serde_json::Value, QuicpulseError> {
    let client = reqwest::Client::new();
    let resp = client.get(url)
        .header("User-Agent", "QuicPulse")
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(QuicpulseError::Request)?;

    if !resp.status().is_success() {
        return Err(QuicpulseError::Argument(format!(
            "HTTP error: {}", resp.status()
        )));
    }

    resp.json().await.map_err(QuicpulseError::Request)
}

/// Fetch text from URL
async fn fetch_text(url: &str) -> Result<String, QuicpulseError> {
    let client = reqwest::Client::new();
    let resp = client.get(url)
        .header("User-Agent", "QuicPulse")
        .send()
        .await
        .map_err(QuicpulseError::Request)?;

    if !resp.status().is_success() {
        return Err(QuicpulseError::Argument(format!(
            "HTTP error: {} - {}", resp.status(), url
        )));
    }

    resp.text().await.map_err(QuicpulseError::Request)
}
