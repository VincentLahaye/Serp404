# Serp404

An open-source desktop application for SEO professionals and webmasters to detect broken pages (404s) among URLs indexed by Google.

Serp404 collects URLs from multiple sources, verifies their Google indexation status, audits their HTTP health, and exports actionable results as CSV.

## Download

[![GitHub Release](https://img.shields.io/github/v/release/VincentLahaye/Serp404?style=for-the-badge)](https://github.com/VincentLahaye/Serp404/releases/latest)

**[Download the latest release](https://github.com/VincentLahaye/Serp404/releases/latest)** — available for Windows (.exe) and macOS (.dmg for Intel and Apple Silicon).

> **macOS users:** The app is not yet notarized. Right-click the app and select "Open" to bypass Gatekeeper.

## Features

- **Multi-source URL collection**: Sitemap.xml auto-parsing, CSV import (Screaming Frog, Ahrefs, SEMrush compatible), Google index discovery via serper.dev
- **Google indexation verification**: Check which URLs are actually indexed by Google using serper.dev API
- **Comprehensive HTTP audit**: Status codes, response times, redirect chains, page titles, empty title detection
- **Real-time progress**: Live-updating dashboard with adjustable concurrency (1-50 threads)
- **Pause/Resume**: Long-running scans can be paused, resumed, or stopped — progress is saved to SQLite
- **CSV export**: Export results with filters (404s, redirects, empty titles, slow pages)
- **BYOK model**: Bring Your Own Key — API keys are stored locally, never transmitted to third parties
- **Cross-platform**: Windows, macOS, Linux

## Screenshots

> Screenshots coming soon

## Tech Stack

- **Frontend**: React, TypeScript, Tailwind CSS, Vite
- **Backend**: Rust (via Tauri v2)
- **Database**: SQLite (embedded, zero config)
- **Desktop**: Tauri v2 (~10-15 MB installed)

## Prerequisites

- [Rust](https://rustup.rs/) (1.70+)
- [Node.js](https://nodejs.org/) (18+)
- System dependencies (Linux only):
  ```bash
  sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
  ```

## Installation

```bash
git clone https://github.com/VincentLahaye/Serp404.git
cd Serp404
npm install
```

## Development

```bash
npm run tauri dev
```

## Build

```bash
npm run tauri build
```

Installers will be generated in `src-tauri/target/release/bundle/`.

## Usage

1. **Configure API key** (optional): Go to Settings and enter your [serper.dev](https://serper.dev) API key
2. **Create a project**: Click "New Project" and enter a domain (e.g., `example.com`)
3. **Collect URLs**: Use one or more sources:
   - Fetch the site's sitemap.xml automatically
   - Upload a CSV file from Screaming Frog or other SEO tools
   - Discover indexed URLs via serper.dev
4. **Verify indexation** (optional): Check which collected URLs are actually in Google's index
5. **Run audit**: Start the HTTP health check with adjustable concurrency
6. **Export results**: Filter and export as CSV for further analysis

## API Keys (BYOK)

Serp404 uses a Bring Your Own Key model. The only external API currently supported is:

- **[serper.dev](https://serper.dev)**: Used for Google index discovery and verification. Free tier includes 2,500 queries. Keys are stored locally in your SQLite database and never sent anywhere except to serper.dev.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'feat: add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.
