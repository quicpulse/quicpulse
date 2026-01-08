# Platform-Specific Resources

This directory contains platform-specific files, documentation, and tooling.

## Directory Structure

### `android-not-supported/`

Contains resources for running QuicPulse on Android via Termux. **Note:** Native Android builds are not officially supported at this time due to Android's Bionic libc incompatibilities with QuicPulse's dependencies.

**Contents:**
- `README.md` - Complete guide for installing and running QuicPulse on Android/Termux using musl static binaries
- `Dockerfile.termux` - Docker image for testing QuicPulse in a Termux environment
- `docker-compose.termux.yml` - Docker Compose configuration for simplified Termux testing
- `test-termux-docker.sh` - Automated test script for Termux Docker environment

**Quick Start:**
See [android-not-supported/README.md](android-not-supported/README.md) for complete instructions.

## Why "android-not-supported"?

While QuicPulse can run on Android devices via Termux using static musl binaries, native Android builds (using Bionic libc) are not officially supported. The Android targets in CI build properly compiled binaries, but they have limitations:

1. **No JavaScript Scripting** - Android builds are compiled without the `javascript` feature due to `rquickjs` library limitations (missing Android bindings)
2. **Bionic libc differences** - Android uses Bionic instead of glibc or musl
3. **Non-standard paths** - Android filesystem layout differs significantly from standard Linux
4. **Certificate management** - Android's certificate store is separate from standard Linux paths
5. **Network restrictions** - Android 9+ blocks cleartext HTTP by default

**Recommendation:** Use the static musl binaries (aarch64-unknown-linux-musl, x86_64-unknown-linux-musl) for Termux instead. They:
- ✅ Include **full JavaScript scripting support**
- ✅ Work reliably with proper SSL_CERT_FILE configuration
- ✅ Are statically linked and self-contained

## Contributing

If you have improvements to Android/Termux support or want to add support for other platforms, please open an issue or pull request.
