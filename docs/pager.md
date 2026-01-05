# Response Pager Reference

Automatic paging of long responses through your preferred pager program.

## Overview

QuicPulse can automatically pipe long responses through a pager (like `less` or `more`) when the output exceeds the terminal height. This makes it easier to navigate through large JSON responses, API documentation, or any long output.

## Quick Start

```bash
# Enable pager for long output
quicpulse --pager httpbin.org/json

# Force pager even for short output
quicpulse --pager --pager-force httpbin.org/get

# Use a specific pager command
quicpulse --pager --pager-cmd="less -R" httpbin.org/json
```

## Configuration

### CLI Options

| Option | Description |
|--------|-------------|
| `--pager` | Enable automatic paging for long output |
| `--pager-force` | Force pager even for short output |
| `--pager-cmd <CMD>` | Specify custom pager command |

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `PAGER` | Default pager program | `less -FRX` |
| `QUICPULSE_PAGER` | QuicPulse-specific pager (overrides `$PAGER`) | - |

### Automatic Paging Behavior

The pager is automatically triggered when:

1. `--pager` flag is enabled
2. Output is sent to a TTY (terminal)
3. Content exceeds the terminal height

The pager is NOT triggered when:

- Output is piped to another command
- Output is redirected to a file
- Content is shorter than terminal height (unless `--pager-force` is used)

## Examples

### Basic Paging

```bash
# Page through large JSON response
quicpulse --pager api.example.com/large-response

# Page through response headers and body
quicpulse --pager -v api.example.com/endpoint
```

### Custom Pager Commands

```bash
# Use less with color support
quicpulse --pager --pager-cmd="less -R" httpbin.org/json

# Use more instead of less
quicpulse --pager --pager-cmd="more" httpbin.org/json

# Use bat for syntax highlighting
quicpulse --pager --pager-cmd="bat --style=plain --language=json" httpbin.org/json

# Use most for split-screen viewing
quicpulse --pager --pager-cmd="most" httpbin.org/json
```

### Configuration File

Add pager settings to `~/.config/quicpulse/config.toml`:

```toml
[defaults]
pager = true
pager_cmd = "less -FRX"
```

## Pager Program Options

### less (Default)

Recommended flags for `less`:

| Flag | Description |
|------|-------------|
| `-F` | Quit if content fits on one screen |
| `-R` | Process ANSI color codes |
| `-X` | Don't clear screen on exit |
| `-S` | Chop long lines instead of wrapping |

```bash
# Recommended less configuration
PAGER="less -FRXS"
```

### Common Pager Programs

| Pager | Install | Notes |
|-------|---------|-------|
| `less` | Pre-installed on most systems | Most common, feature-rich |
| `more` | Pre-installed | Basic paging |
| `bat` | `brew install bat` | Syntax highlighting |
| `most` | `brew install most` | Multi-window support |
| `vim` | Pre-installed | Full editor as pager |

## Integration with Other Options

### With Pretty Printing

```bash
# Pager with formatted JSON
quicpulse --pager --pretty=all api.example.com/json

# Pager with colors but no formatting
quicpulse --pager --pretty=colors api.example.com/json
```

### With Filtering

```bash
# Page filtered output
quicpulse --pager --filter='.data[]' api.example.com/large-list
```

### With Headers

```bash
# Page verbose output (request + response)
quicpulse --pager -v api.example.com/endpoint

# Page response headers + body
quicpulse --pager --print=hb api.example.com/endpoint
```

## Troubleshooting

### Pager Not Triggering

1. Ensure terminal is a TTY (`isatty(stdout)`)
2. Check if `--pager` flag is enabled
3. Verify content exceeds terminal height
4. Check `$PAGER` environment variable

### Color Codes Showing as Text

Use a pager that supports ANSI codes:

```bash
# less with -R flag
quicpulse --pager --pager-cmd="less -R" endpoint

# Or set in environment
export PAGER="less -R"
```

### Screen Clearing on Exit

Add `-X` flag to less:

```bash
quicpulse --pager --pager-cmd="less -RX" endpoint
```

### Binary Content

The pager may not work well with binary responses. Use `--download` for binary files:

```bash
# Download binary instead of paging
quicpulse -d https://example.com/file.zip
```

## Best Practices

1. **Use `-F` with less** - Auto-exit if content fits on screen
2. **Enable colors with `-R`** - Preserve syntax highlighting
3. **Use `-X` to keep output** - Don't clear screen on pager exit
4. **Configure globally** - Set `$PAGER` for consistent behavior
5. **Skip for pipelines** - Pager is automatically disabled when piping

## API Reference

### PagerConfig

```rust
pub struct PagerConfig {
    /// Whether paging is enabled
    pub enabled: bool,
    /// Custom pager command (overrides $PAGER)
    pub command: Option<String>,
}
```

### Functions

```rust
/// Get the pager command from environment or default
pub fn get_pager_command() -> String;

/// Determine if paging should occur
pub fn should_page(content: &str, is_tty: bool, forced: bool) -> bool;

/// Write content through pager
pub fn write_with_pager<W: Write>(
    output: &mut W,
    content: &str,
    config: &PagerConfig,
    is_tty: bool
) -> io::Result<()>;
```

---

See also:
- [README.md](../README.md) - CLI reference
- [workflow.md](workflow.md) - Workflow reference
