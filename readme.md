# Etsu

![etsu](https://github.com/user-attachments/assets/014ef834-63bc-42a8-a396-e158c8012044)


An elegant personal spyware. (JK, it tracks silly metrics)

## Features

- Tracks keypresses, mouse clicks, scroll steps, and mouse distance traveled
- Local SQLite storage with optional PostgreSQL syncing
- Minimal resource usage
- Simple configuration
- Runs as a background service/daemon

## Installation

### Package Installers

Download platform-specific packages from the [Releases](https://github.com/seatedro/etsu/releases) page:
- **macOS**: `.app` bundle or `.pkg` installer
- **Linux**: `.deb`, `.rpm`, or `.AppImage`
- **Windows**: `.msi` installer

### Build from Source

```bash
# Clone the repository
git clone https://github.com/seatedro/etsu.git
cd etsu

# Build in release mode
cargo build --release

# The binary will be available at target/release/etsu
```

## Configuration

Etsu uses a TOML configuration file. Copy the example configuration:

```bash
cp config.example.toml config.toml
```

Edit the `config.toml` file to adjust settings.

### Configuration File Locations

The configuration file is searched in these locations:

- **macOS**: `~/Library/Application Support/com.seatedro.etsu/config.toml`
- **Linux**: `~/.config/etsu/config.toml`
- **Windows**: `%APPDATA%\seatedro\etsu\config.toml`

## Usage

### Running as a Service/Daemon

Etsu is designed to run as a background service:

#### macOS
```bash
# Install as a launchd service
sudo cp extras/macos/com.seatedro.etsu.plist /Library/LaunchDaemons/
sudo launchctl load -w /Library/LaunchDaemons/com.seatedro.etsu.plist
```

#### Linux (systemd)
```bash
# Install as a systemd service
sudo cp extras/linux/etsu.service /etc/systemd/system/
sudo systemctl enable etsu
sudo systemctl start etsu
```

#### Windows
```
# The installer automatically registers as a Windows service
# Manual registration:
sc.exe create Etsu binPath="C:\Program Files\Etsu\etsu.exe" start=auto
sc.exe start Etsu
```

### Running Manually

You can also run Etsu directly:

```bash
# On macOS/Linux
./etsu

# On Windows
etsu.exe
```

### Viewing Statistics

Etsu stores metrics in a local SQLite database located at:

- **macOS**: `~/Library/Application Support/com.seatedro.etsu/metrics.db`
- **Linux**: `~/.local/share/etsu/metrics.db`
- **Windows**: `%LOCALAPPDATA%\seatedro\etsu\metrics.db`

## License

[MIT](LICENSE)
