# Running QuicPulse on Android with Termux

QuicPulse provides static musl binaries that work on Android via [Termux](https://termux.dev/). This guide covers installation, troubleshooting, and Docker testing.

**Note:** Native Android builds (using the `*-linux-android` targets) are built without JavaScript scripting support due to `rquickjs` library limitations on Android. All other features remain fully functional.

## Table of Contents

- [Quick Installation](#quick-installation)
- [Detailed Setup](#detailed-setup)
- [Troubleshooting](#troubleshooting)
- [Docker Testing](#docker-testing)
- [Common Issues](#common-issues)
- [Limitations](#limitations)

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

## Debug Mode

QuicPulse provides comprehensive debugging to help diagnose network issues on Android/Termux.

### Basic Debug Mode

Shows human-readable debug output with platform detection, timing, and request/response details:

```bash
quicpulse --debug GET https://example.com
```

**Debug output includes:**
- Platform detection (OS, Android/Termux identification)
- Certificate paths (shows SSL_CERT_FILE location)
- HTTP version negotiation (HTTP/1.1, HTTP/2, HTTP/3)
- TLS handshake details (cipher suites, ALPN protocol)
- DNS resolution and connection details
- Request and response headers
- Total request timing
- Detailed error context with phase identification

### JSON Debug Mode

For parsing by tools or CI/CD pipelines, use `--debug-json` to output structured JSON to stderr:

```bash
quicpulse --debug-json GET https://example.com 2>debug.json
```

Each log line is a complete JSON object with structured fields for automated analysis.

### Common Android/Termux Issues Detected by Debug Mode

1. **"error sending request" with HTTP URLs**
   - **Cause:** Android 9+ blocks cleartext HTTP traffic by default
   - **Solution:** Use HTTPS instead
   - **Debug:** `--debug` will show a warning about cleartext blocking

2. **SSL certificate errors**
   - **Cause:** Missing or incorrect SSL_CERT_FILE environment variable
   - **Solution:** Export the certificate path (see below)
   - **Debug:** `--debug` shows detected certificate paths

3. **Connection timeouts**
   - **Cause:** Network restrictions or firewall rules
   - **Debug:** `--debug` shows which phase failed (DNS, connect, TLS, request)

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

For testing QuicPulse in a Termux Docker environment, see [`Dockerfile.termux`](Dockerfile.termux) in this directory.

### Quick Docker Test

```bash
# Build the Docker image (run from repository root)
docker build -f platform/android-not-supported/Dockerfile.termux -t quicpulse-termux .

# Run a test (note: requires --network host)
docker run --rm --network host \
  -v $(pwd)/target/aarch64-unknown-linux-musl/debug/quicpulse:/tmp/quicpulse:ro \
  quicpulse-termux \
  /tmp/quicpulse https://httpbin.org/get
```

Or use the automated test script:
```bash
./platform/android-not-supported/test-termux-docker.sh
```

### Using Docker Compose (Recommended for Contributors)

**Docker Compose solves the `--network host` requirement automatically**, making it easier for contributors who git clone the repo.

**Why Docker Compose?**
- No need to remember `--network host` flag
- Codifies all required flags and volume mounts
- One command for testing: `docker compose -f platform/android-not-supported/docker-compose.termux.yml run test`
- Prevents common mistakes when running manually

**Prerequisites:**
```bash
# Build the musl binary first (run from repository root)
cross build --target aarch64-unknown-linux-musl
```

**Usage (run from repository root):**

1. **Run automated tests:**
   ```bash
   docker compose -f platform/android-not-supported/docker-compose.termux.yml run test
   ```

2. **Interactive shell:**
   ```bash
   docker compose -f platform/android-not-supported/docker-compose.termux.yml run shell
   # Inside container:
   /tmp/quicpulse --version
   /tmp/quicpulse https://httpbin.org/get
   ```

3. **Single command:**
   ```bash
   docker compose -f platform/android-not-supported/docker-compose.termux.yml run quicpulse /tmp/quicpulse https://httpbin.org/get
   ```

**What it does:**
- Automatically uses `network_mode: host` (solves DNS/networking issues)
- Mounts the binary at `/tmp/quicpulse` (read-only)
- Sets `SSL_CERT_FILE` environment variable
- Provides certificates from Alpine Linux

**For CI/CD or manual testing without Compose,** use `./platform/android-not-supported/test-termux-docker.sh` or the docker commands below.

### Understanding the dnsmasq Warning

When running the Termux Docker container, you'll see:
```
[!] Failed to start dnsmasq, host name resolution may fail.
```

**What is dnsmasq?**
- `dnsmasq` is a lightweight DNS forwarder and DHCP server
- The Termux Docker container tries to start it on initialization
- It's used for local DNS caching and resolution

**Why does it fail in Docker?**

The Termux Docker container has architectural limitations:
1. **Missing privileges:** DNS services require network capabilities that Docker's default security doesn't allow
2. **Port conflicts:** dnsmasq needs port 53, which may conflict with host DNS
3. **Network isolation:** Bridge networking in the container is fundamentally broken

**Does this affect QuicPulse?**

In the Docker container: **Yes, without workarounds**
- Bridge networking: ❌ DNS fails, QuicPulse can't resolve hostnames
- Host networking: ✅ DNS works, QuicPulse works perfectly

On real Android devices: **No, not at all**
- Real Termux uses Android's system DNS resolver
- No dnsmasq required or used
- DNS resolution works normally

### Docker Environment Issues & Solutions

The Termux Docker container (`termux/termux-docker`) has three major issues that don't exist on real devices:

| Issue | Docker Container | Real Android/Termux | Solution (Docker Only) |
|-------|------------------|---------------------|------------------------|
| **DNS Resolution** | ❌ Broken (dnsmasq fails) | ✅ Works (uses Android DNS) | Use `--network host` |
| **Bridge Networking** | ❌ Broken (can't route) | ✅ Works normally | Use `--network host` |
| **CA Certificates** | ❌ Missing at `/etc/ssl/certs/` | ✅ Present at `$PREFIX/etc/tls/` | Copy from Alpine + set `SSL_CERT_FILE` |

### How Dockerfile.termux Solves These Issues

Our `Dockerfile.termux` includes three critical workarounds:

#### 1. Host Networking (Solves DNS + Networking)

```bash
docker run --network host ...
```

**What this does:**
- Bypasses Docker's broken bridge networking
- Uses the host system's network stack directly
- Gives container access to host's DNS resolver
- No isolation from host network (acceptable for testing)

**Why bridge networking fails:**
```bash
# This WILL NOT WORK - DNS fails
docker run --rm quicpulse-termux /tmp/quicpulse https://httpbin.org/get
# Error: DNS resolution fails

# This WORKS - uses host DNS
docker run --rm --network host quicpulse-termux /tmp/quicpulse https://httpbin.org/get
# Success!
```

#### 2. Alpine CA Certificates (Solves TLS)

```dockerfile
# Multi-stage build copies Alpine's certificates
FROM alpine:latest AS certs
RUN tar -czf /tmp/certs.tar.gz /etc/ssl/certs

FROM termux/termux-docker:aarch64
COPY --from=certs /tmp/certs.tar.gz /tmp/alpine-certs.tar.gz
RUN cd / && tar -xzf /tmp/alpine-certs.tar.gz
```

**Why this is needed:**
- Termux Docker lacks `/etc/ssl/certs/` directory
- QuicPulse (sideloaded binary) expects standard Linux paths
- Alpine Linux has a complete, up-to-date CA bundle

#### 3. Environment Variables (Tells rustls where certs are)

```dockerfile
ENV SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt
ENV SSL_CERT_DIR=/etc/ssl/certs
```

**How this works:**
- `rustls-native-certs` checks `SSL_CERT_FILE` first
- Bypasses platform-specific path detection
- Points directly to Alpine certificates we installed

### Real Android/Termux Doesn't Need These Workarounds

**Important:** If you're using QuicPulse on an actual Android device with Termux, you only need:

```bash
# Install certificates (one time)
pkg install ca-certificates

# Set environment variable (permanent)
echo 'export SSL_CERT_FILE=$PREFIX/etc/tls/cert.pem' >> ~/.bashrc
source ~/.bashrc

# Done! No networking workarounds needed.
```

Real Termux on Android:
- ✅ Has working DNS (uses Android's system resolver)
- ✅ Has working networking (uses Android's network stack)
- ✅ Has CA certificates at `$PREFIX/etc/tls/`
- ✅ No dnsmasq warning (dnsmasq isn't used)
- ✅ No special flags needed

### Testing Matrix

| Environment | DNS | Networking | TLS | Solution |
|-------------|-----|------------|-----|----------|
| **Docker (bridge)** | ❌ | ❌ | ❌ | Don't use! |
| **Docker (host net)** | ✅ | ✅ | ✅ | Use `--network host` + Dockerfile.termux |
| **Real Android** | ✅ | ✅ | ✅ | Just set `SSL_CERT_FILE` |
| **Alpine Linux** | ✅ | ✅ | ✅ | Works out of the box |

### Why the Docker Container is Still Useful

Despite its limitations, the Termux Docker container is valuable for:
- **CI/CD testing:** Automated testing of ARM64 musl binaries
- **Development:** Quick testing without Android device
- **Reproducibility:** Consistent environment across machines
- **Documentation:** Proving the binary works on ARM64 Linux

**Just remember:** The Docker environment is **more restrictive** than real Termux. If it works in Docker, it will definitely work on real Android devices.

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

- **Documentation:** [docs/](../../docs/)
- **Issues:** [GitHub Issues](https://github.com/quicpulse/quicpulse/issues)
- **Termux Issues:**
  - [Termux CA Certificates #1546](https://github.com/termux/termux-packages/issues/1546)
  - [Termux TLS Verification #4893](https://github.com/termux/termux-app/issues/4893)

---

## Limitations

### Android Native Builds

QuicPulse binaries built for Android targets (`*-linux-android`) have the following limitations compared to standard Linux builds:

**Not Available:**
- ❌ **JavaScript Scripting** - The `rquickjs` library does not support Android platforms due to missing precompiled bindings
  - Cannot use `--script` flag with JavaScript files
  - JavaScript-based request/response processing unavailable

**Fully Available:**
- ✅ **Rune Scripting** - Native Rust scripting language works perfectly
- ✅ **All HTTP Features** - HTTP/1.1, HTTP/2, HTTP/3, WebSocket, gRPC, GraphQL
- ✅ **Authentication** - All auth methods (Basic, Digest, Bearer, OAuth2, AWS SigV4)
- ✅ **All Other Features** - Sessions, workflows, filtering, assertions, etc.

### Musl Static Binaries (Recommended for Termux)

The musl static binaries (aarch64-unknown-linux-musl, x86_64-unknown-linux-musl) include **full JavaScript support** and are recommended for Termux users. These binaries:
- ✅ Include all features including JavaScript scripting
- ✅ Work reliably in Termux with proper SSL_CERT_FILE configuration
- ✅ Are statically linked and self-contained

**Recommendation:** Use the musl binaries (`quicpulse-linux-arm64-musl.tar.gz`) for Termux instead of the Android-specific builds.

---

## Additional Resources

- [Termux Wiki](https://wiki.termux.com/)
- [Termux GitHub](https://github.com/termux/termux-app)
- [QuicPulse Documentation](https://github.com/quicpulse/quicpulse/tree/main/docs)
- [rustls Documentation](https://docs.rs/rustls/)
