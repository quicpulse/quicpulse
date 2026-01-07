# Running QuicPulse on Android with Termux

QuicPulse provides static musl binaries that work perfectly on Android via [Termux](https://termux.dev/). This guide covers installation, troubleshooting, and Docker testing.

## Table of Contents

- [Quick Installation](#quick-installation)
- [Detailed Setup](#detailed-setup)
- [Troubleshooting](#troubleshooting)
- [Docker Testing](#docker-testing)
- [Common Issues](#common-issues)

---

## Quick Installation

### 1. Install Termux

Download Termux from [F-Droid](https://f-droid.org/packages/com.termux/) (recommended) or [GitHub Releases](https://github.com/termux/termux-app/releases).

**Note:** The Google Play Store version is deprecated and may not work correctly.

### 2. Install Required Packages

```bash
pkg update
pkg install wget tar ca-certificates
```

### 3. Download QuicPulse

Choose the appropriate binary for your device:

**ARM64 (most modern Android devices):**
```bash
wget https://github.com/quicpulse/quicpulse/releases/latest/download/quicpulse-linux-arm64-musl.tar.gz
tar -xzf quicpulse-linux-arm64-musl.tar.gz
chmod +x quicpulse
mv quicpulse $PREFIX/bin/
```

**x86_64 (Android emulators):**
```bash
wget https://github.com/quicpulse/quicpulse/releases/latest/download/quicpulse-linux-x86_64-musl.tar.gz
tar -xzf quicpulse-linux-x86_64-musl.tar.gz
chmod +x quicpulse
mv quicpulse $PREFIX/bin/
```

### 4. Configure SSL Certificate Path

**CRITICAL for sideloaded binaries:** QuicPulse is built with `rustls-native-certs`, which expects certificates at standard Linux paths. Termux stores certificates in a non-standard location, so we need to tell rustls where to find them.

```bash
# Add to your shell profile
echo 'export SSL_CERT_FILE=$PREFIX/etc/tls/cert.pem' >> ~/.bashrc
source ~/.bashrc
```

For other shells:
```bash
# For zsh
echo 'export SSL_CERT_FILE=$PREFIX/etc/tls/cert.pem' >> ~/.zshrc
source ~/.zshrc

# For fish
echo 'set -gx SSL_CERT_FILE $PREFIX/etc/tls/cert.pem' >> ~/.config/fish/config.fish
```

**Why this is necessary:** Sideloaded binaries (like QuicPulse's static musl build) look for certificates at `/etc/ssl/certs/`, but Termux stores them at `$PREFIX/etc/tls/`. The `SSL_CERT_FILE` environment variable tells rustls where to actually find them.

### 5. Verify Installation

```bash
quicpulse --version
```

### 6. Test with a Request

```bash
quicpulse https://httpbin.org/get
```

If you get certificate errors, verify the environment variable is set:
```bash
echo $SSL_CERT_FILE
# Should output: /data/data/com.termux/files/usr/etc/tls/cert.pem
```

---

## Detailed Setup

### Understanding the Binary

QuicPulse musl binaries are:
- **Statically linked** - No glibc dependency
- **Self-contained** - All dependencies bundled
- **Portable** - Works across different Android versions
- **Optimized** - Built with link-time optimization

### File Locations in Termux

Termux uses a non-standard filesystem layout:

| Standard Path | Termux Path |
|---------------|-------------|
| `/usr/bin/` | `$PREFIX/bin/` (`/data/data/com.termux/files/usr/bin/`) |
| `/etc/` | `$PREFIX/etc/` (`/data/data/com.termux/files/usr/etc/`) |
| `/tmp/` | `$PREFIX/tmp/` (`/data/data/com.termux/files/usr/tmp/`) |

Always use `$PREFIX` in scripts for portability.

### SSL/TLS Certificates

QuicPulse uses `rustls-native-certs` which loads CA certificates from the system. The `ca-certificates` package provides these.

**Installation:**
```bash
pkg install ca-certificates
```

**Certificate location:**
```bash
ls -la $PREFIX/etc/tls/certs/
```

**Manual verification:**
```bash
# Should show 100+ certificates
ls $PREFIX/etc/tls/certs/*.pem | wc -l
```

### Adding Custom/Self-Signed Certificates

If you need to trust custom certificates (e.g., for corporate proxies, self-signed servers, or internal CAs), Termux provides the `add-trusted-certificate` script.

**Method 1: Using add-trusted-certificate (Recommended)**
```bash
# Download or obtain your certificate file (PEM, CRT, CER format)
# Example: corporate-ca.crt

# Add it to Termux's trust store
add-trusted-certificate corporate-ca.crt

# Verify it was added
ls $PREFIX/etc/tls/certs/ | grep corporate
```

**Method 2: Manual Installation**
```bash
# Copy certificate to the certs directory
cp your-certificate.pem $PREFIX/etc/tls/certs/

# Set proper permissions
chmod 644 $PREFIX/etc/tls/certs/your-certificate.pem

# Rehash certificates (if openssl-tool is installed)
c_rehash $PREFIX/etc/tls/certs/
```

**Supported formats:**
- `.pem` - Privacy Enhanced Mail (preferred)
- `.crt` - Certificate file
- `.cer` - Certificate file (alternative)

**Important:** After adding certificates, restart your shell or run `source ~/.bashrc` to ensure environment variables are refreshed.

**For Android system certificates:** Note that Termux does not automatically trust certificates installed in Android's system settings. You must manually export and add them using the methods above.

---

## Troubleshooting

### Issue 1: SSL/TLS Certificate Errors

**Symptoms:**
```
Error: Request error: tls: failed to verify certificate
Error: Request error: x509: certificate signed by unknown authority
Error: Request error: builder error
```

**Primary Solution - Set SSL_CERT_FILE (Issue #4893):**

This is the **most common cause** for sideloaded binaries like QuicPulse. The binary expects certificates at standard Linux paths but Termux stores them elsewhere.

```bash
# Set the environment variable
export SSL_CERT_FILE=$PREFIX/etc/tls/cert.pem

# Make it permanent
echo 'export SSL_CERT_FILE=$PREFIX/etc/tls/cert.pem' >> ~/.bashrc
source ~/.bashrc

# Verify it's set
echo $SSL_CERT_FILE
# Should output: /data/data/com.termux/files/usr/etc/tls/cert.pem

# Test again
quicpulse https://httpbin.org/get
```

**Secondary Solution - Install/Update Certificates:**

If `SSL_CERT_FILE` is set but you still get errors:

```bash
# Update package lists and install certificates
pkg update
pkg install ca-certificates

# Verify certificates are installed
ls -la $PREFIX/etc/tls/certs/
```

**If still failing:**
1. **Check environment variable:** Run `echo $SSL_CERT_FILE` - must be set!
2. **Check system date/time:** Incorrect time causes cert validation failures
   ```bash
   date
   # Should show correct current time
   ```
3. **Try with HTTP first** to isolate the issue:
   ```bash
   quicpulse http://httpbin.org/get
   ```
4. **Check certificate file exists:**
   ```bash
   ls -la $PREFIX/etc/tls/cert.pem
   # Should show a file, not an error
   ```
5. **Check Termux app permissions:** Storage, Network (in Android settings)
6. **For self-signed/custom certificates:** See [Adding Custom Certificates](#adding-customself-signed-certificates) above

**Related Issues:**
- [termux/termux-app#4893](https://github.com/termux/termux-app/issues/4893) - TLS verification for sideloaded binaries
- [termux/termux-packages#1546](https://github.com/termux/termux-packages/issues/1546) - Custom certificate support

### Issue 2: DNS Resolution Failures

**Symptoms:**
```
Error: Request error: dns error: failed to lookup address
```

**Solution:**
```bash
# Test DNS resolution
ping -c 1 google.com

# If DNS doesn't work, check network connectivity
termux-wifi-connectioninfo

# Try using an IP address directly
quicpulse http://93.184.215.14
```

### Issue 3: Binary Won't Execute

**Symptoms:**
```
bash: ./quicpulse: cannot execute binary file
```

**Causes:**
- Wrong architecture (downloaded x86_64 for ARM device or vice versa)
- Downloaded glibc version instead of musl version

**Solution:**
```bash
# Check your device architecture
uname -m
# arm64 or aarch64 → Use linux-arm64-musl
# x86_64 → Use linux-x86_64-musl

# Verify the binary
file quicpulse
# Should say: "ELF 64-bit LSB executable, ARM aarch64 ... statically linked"

# Make sure you downloaded the -musl version
ls -la quicpulse*
```

### Issue 4: Permission Denied

**Symptoms:**
```
bash: ./quicpulse: Permission denied
```

**Solution:**
```bash
chmod +x quicpulse
```

### Issue 5: Network Access Issues

**Symptoms:**
```
Error: Request error: Network unreachable
```

**Solution:**
1. Check Termux has network permissions in Android settings
2. Try switching between WiFi and mobile data
3. Test with another tool:
   ```bash
   curl https://httpbin.org/get
   ```

---

## Docker Testing

For testing QuicPulse in a Termux Docker environment, see [`Dockerfile.termux`](../Dockerfile.termux) in the repository root.

### Quick Docker Test

```bash
# Build the Docker image
docker build -f Dockerfile.termux -t quicpulse-termux .

# Run a test
docker run --rm quicpulse-termux /tmp/quicpulse https://httpbin.org/get
```

### Docker Environment Issues

The Termux Docker container (`termux/termux-docker`) has known issues:
- ❌ Broken DNS resolution (dnsmasq fails)
- ❌ Missing CA certificates by default
- ❌ Broken bridge networking

The `Dockerfile.termux` includes workarounds:
- Uses host networking
- Installs CA certificates from Alpine
- Sets proper environment variables

**Important:** Real Android devices don't have these issues. The Docker container is useful for CI/CD testing but doesn't represent real Termux behavior.

---

## Common Issues

### "The standard Linux binaries don't work"

You downloaded the glibc version (`linux-x86_64-gnu` or `linux-arm64-gnu`) instead of the musl version.

**Fix:** Download the `-musl` variant:
- `quicpulse-linux-arm64-musl.tar.gz` ✅
- `quicpulse-linux-x86_64-musl.tar.gz` ✅

### "HTTPS doesn't work but HTTP does"

Missing CA certificates.

**Fix:**
```bash
pkg install ca-certificates
```

### "I get 'command not found' after installation"

The binary isn't in your PATH.

**Fix:**
```bash
# Make sure it's in $PREFIX/bin
mv quicpulse $PREFIX/bin/

# Or add current directory to PATH temporarily
export PATH=$PATH:.
./quicpulse --version
```

### "Downloads are slow"

Termux mirrors can be slow depending on your location.

**Fix:**
```bash
# Change to a faster mirror
termux-change-repo
```

---

## Advanced Usage

### Shell Aliases

Add to `~/.bashrc` or `~/.zshrc`:

```bash
# Shortcuts for common operations
alias qp='quicpulse'
alias qpget='quicpulse GET'
alias qppost='quicpulse POST'
alias qpjson='quicpulse --json'
```

### Integration with Termux:API

Combine QuicPulse with Termux:API for powerful automations:

```bash
# Get location and send to API
LAT=$(termux-location | jq -r '.latitude')
LON=$(termux-location | jq -r '.longitude')
quicpulse POST https://api.example.com/location lat:=$LAT lon:=$LON
```

### Scheduled Tasks with Cron

```bash
# Install cronie
pkg install cronie

# Edit crontab
crontab -e

# Example: Check API every hour
0 * * * * quicpulse https://api.example.com/health >> $PREFIX/tmp/health-check.log 2>&1
```

---

## Performance Notes

### Binary Size

The debug build is ~400MB. Release builds (from GitHub Releases) are significantly smaller due to:
- Strip symbols
- Link-time optimization (LTO)
- Optimized compilation (`-O3`)

### Memory Usage

QuicPulse is designed to be memory-efficient:
- Streaming downloads/uploads
- Minimal allocations
- Efficient request/response handling

Typical memory usage: 10-30MB depending on request complexity.

### Battery Impact

HTTP/3 (QUIC) may use slightly more battery than HTTP/2 due to UDP encryption overhead. For battery-constrained scenarios:

```bash
# Force HTTP/2
quicpulse --http2 https://example.com

# Or HTTP/1.1
quicpulse --http1.1 https://example.com
```

---

## Security Considerations

### HTTPS/TLS

Always use HTTPS for sensitive data. QuicPulse uses rustls (memory-safe Rust TLS implementation):
- ✅ No OpenSSL vulnerabilities
- ✅ Modern cipher suites
- ✅ Certificate validation enabled by default

### Certificate Pinning

For maximum security, verify server certificates:

```bash
# Disable for testing only (not recommended for production)
quicpulse --verify no https://self-signed.example.com
```

### Storing Credentials

Never store credentials in shell history. Use environment variables:

```bash
# Set environment variable
export API_TOKEN="secret_token_here"

# Use in request
quicpulse https://api.example.com Authorization:"Bearer $API_TOKEN"
```

Or use `.netrc`:

```bash
# ~/.netrc
machine api.example.com
login user
password secret
```

Then:
```bash
quicpulse --auth-type basic https://api.example.com
```

---

## Getting Help

- **Documentation:** [docs/](../docs/)
- **Issues:** [GitHub Issues](https://github.com/quicpulse/quicpulse/issues)
- **Termux Issues:**
  - [Termux CA Certificates #1546](https://github.com/termux/termux-packages/issues/1546)
  - [Termux TLS Verification #4893](https://github.com/termux/termux-app/issues/4893)

---

## Additional Resources

- [Termux Wiki](https://wiki.termux.com/)
- [Termux GitHub](https://github.com/termux/termux-app)
- [QuicPulse Documentation](https://github.com/quicpulse/quicpulse/tree/main/docs)
- [rustls Documentation](https://docs.rs/rustls/)
