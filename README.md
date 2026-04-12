# The Last Place You Look

A local-first desktop app to manage and organize messy digital archives across mounted external drives.

## Development Workflow

### Prerequisites
- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) (v20+)
- [Tauri CLI 2.0](https://v2.tauri.app/) (`cargo install tauri-cli --version "^2.0"`)
- Visual Studio C++ Build Tools (required for Tauri on Windows)

### Running Locally
To run the app with the React frontend and Rust backend:
```powershell
npm install
npm run tauri dev
```

### Debugging
The app uses structured `tracing` logs. To view debug logs from the rust backend, set the `RUST_LOG` environment variable:
```powershell
$env:RUST_LOG="debug"
npm run tauri dev
```

### Database Management
The application stores its catalog in a local embedded SQLite database.
- **Location (Windows):** `%APPDATA%\com.tlpyl.app\tlpyl.db`
- **Resetting state:** To wipe the catalog and start fresh, simply delete the `tlpyl.db` file while the app is closed.

### Building for Release
```powershell
npm run tauri build
```
