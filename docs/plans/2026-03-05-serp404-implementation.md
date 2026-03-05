# Serp404 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a Tauri v2 desktop app that discovers Google-indexed URLs, verifies indexation, audits HTTP health, and exports results as CSV.

**Architecture:** Tauri v2 with React/Vite/Tailwind frontend and Rust backend. SQLite for persistence. Tauri events for real-time progress. tokio + reqwest for concurrent HTTP. Three URL sources: serper.dev, sitemap.xml, CSV upload.

**Tech Stack:** Tauri v2, React 19, TypeScript, Vite, Tailwind CSS, Rust, rusqlite, reqwest, tokio, serde, quick-xml, csv crate

---

## Task 1: Scaffold Tauri v2 project

**Files:**
- Create: entire project scaffold via `create-tauri-app`
- Modify: `package.json` (add tailwind), `tailwind.config.ts`, `src/index.css`

**Step 1: Create Tauri project**

Run:
```bash
cd /home/vincent/Serp404
npm create tauri-app@latest . -- --template react-ts --manager npm
```

If interactive prompts appear, select: TypeScript, React, npm.

If the directory is not empty, move existing files (README.md, LICENSE, docs/) aside, scaffold, then restore them.

**Step 2: Install and verify**

```bash
npm install
npm run tauri dev
```

Expected: A Tauri window opens with the default React template. Close it.

**Step 3: Add Tailwind CSS**

```bash
npm install -D tailwindcss @tailwindcss/vite
```

Update `vite.config.ts` to add the tailwind plugin:
```ts
import tailwindcss from "@tailwindcss/vite";
// add tailwindcss() to plugins array
```

Replace `src/index.css` content with:
```css
@import "tailwindcss";
```

**Step 4: Verify Tailwind works**

Replace `src/App.tsx` with a minimal dark-mode test:
```tsx
function App() {
  return (
    <div className="min-h-screen bg-[#0a0a0f] text-white flex items-center justify-center">
      <h1 className="text-3xl font-bold">Serp404</h1>
    </div>
  );
}
export default App;
```

Run `npm run tauri dev`. Expected: Dark window with white "Serp404" text.

**Step 5: Commit**

```bash
git add -A
git commit -m "feat: scaffold Tauri v2 project with React, TypeScript, Tailwind"
```

---

## Task 2: SQLite database layer

**Files:**
- Modify: `src-tauri/Cargo.toml` (add rusqlite, serde, uuid)
- Create: `src-tauri/src/db.rs`
- Create: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/lib.rs` (register db module)

**Step 1: Add Rust dependencies**

In `src-tauri/Cargo.toml` under `[dependencies]`:
```toml
rusqlite = { version = "0.32", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4"] }
```

**Step 2: Write models**

Create `src-tauri/src/models.rs`:
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub domain: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UrlEntry {
    pub id: String,
    pub project_id: String,
    pub url: String,
    pub source: String,
    pub indexed_status: String,
    pub http_status: Option<i32>,
    pub response_time_ms: Option<i64>,
    pub title: Option<String>,
    pub redirect_chain: Option<String>,
    pub error: Option<String>,
    pub checked_at: Option<String>,
}
```

**Step 3: Write database module with migrations**

Create `src-tauri/src/db.rs`:
```rust
use rusqlite::{Connection, Result};
use std::sync::Mutex;

pub struct Database {
    pub conn: Mutex<Connection>,
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn: Mutex::new(conn) };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                domain TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'created',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS urls (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL REFERENCES projects(id),
                url TEXT NOT NULL,
                source TEXT NOT NULL,
                indexed_status TEXT NOT NULL DEFAULT 'unknown',
                http_status INTEGER,
                response_time_ms INTEGER,
                title TEXT,
                redirect_chain TEXT,
                error TEXT,
                checked_at TEXT,
                UNIQUE(project_id, url)
            );
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
        ")?;
        Ok(())
    }
}
```

**Step 4: Wire up in lib.rs**

Register the `db` and `models` modules in `src-tauri/src/lib.rs`. Initialize the Database in Tauri's `setup` hook and manage it as Tauri state:
```rust
mod db;
mod models;

use db::Database;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_dir = app.path().app_data_dir().expect("failed to get app data dir");
            std::fs::create_dir_all(&app_dir).ok();
            let db_path = app_dir.join("serp404.db");
            let database = Database::new(db_path.to_str().unwrap())
                .expect("failed to initialize database");
            app.manage(database);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**Step 5: Test compilation**

```bash
cd src-tauri && cargo build
```

Expected: Compiles without errors.

**Step 6: Commit**

```bash
git add -A
git commit -m "feat: add SQLite database layer with migrations"
```

---

## Task 3: Project CRUD commands

**Files:**
- Create: `src-tauri/src/commands/mod.rs`
- Create: `src-tauri/src/commands/projects.rs`
- Modify: `src-tauri/src/lib.rs` (register commands)

**Step 1: Create project commands**

Create `src-tauri/src/commands/mod.rs`:
```rust
pub mod projects;
```

Create `src-tauri/src/commands/projects.rs` with these Tauri commands:
- `create_project(domain: String) -> Project` — generates UUID, inserts into DB, returns Project
- `list_projects() -> Vec<Project>` — returns all projects ordered by created_at DESC
- `get_project(id: String) -> Project` — returns single project
- `delete_project(id: String)` — deletes project and all its URLs

Each command takes `State<Database>` as parameter.

**Step 2: Register commands in lib.rs**

Add to `invoke_handler`:
```rust
.invoke_handler(tauri::generate_handler![
    commands::projects::create_project,
    commands::projects::list_projects,
    commands::projects::get_project,
    commands::projects::delete_project,
])
```

**Step 3: Test compilation**

```bash
cd src-tauri && cargo build
```

**Step 4: Commit**

```bash
git add -A
git commit -m "feat: add project CRUD Tauri commands"
```

---

## Task 4: Settings commands

**Files:**
- Create: `src-tauri/src/commands/settings.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: Create settings commands**

Create `src-tauri/src/commands/settings.rs`:
- `get_setting(key: String) -> Option<String>`
- `set_setting(key: String, value: String)`
- `get_all_settings() -> HashMap<String, String>`
- `test_serper_key(api_key: String) -> Result<bool, String>` — makes a test request to `https://google.serper.dev/search` with a trivial query to verify the key works

**Step 2: Register in lib.rs, build, commit**

```bash
git add -A
git commit -m "feat: add settings CRUD and serper key validation"
```

---

## Task 5: Sitemap parser

**Files:**
- Modify: `src-tauri/Cargo.toml` (add reqwest, tokio, quick-xml)
- Create: `src-tauri/src/crawler/mod.rs`
- Create: `src-tauri/src/crawler/sitemap.rs`
- Create: `src-tauri/src/commands/collection.rs`

**Step 1: Add HTTP/XML dependencies**

In `src-tauri/Cargo.toml`:
```toml
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
tokio = { version = "1", features = ["full"] }
quick-xml = "0.37"
```

**Step 2: Write sitemap parser**

Create `src-tauri/src/crawler/sitemap.rs`:
- `pub async fn fetch_sitemap(domain: &str) -> Result<Vec<String>>` — fetches `https://{domain}/sitemap.xml`, parses XML, extracts `<loc>` URLs
- Handles nested sitemaps (`<sitemap><loc>...</loc></sitemap>`) recursively
- Handles sitemap index files
- Returns deduplicated list of URLs

**Step 3: Write unit test**

Add `#[cfg(test)]` module in sitemap.rs:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sitemap_xml() {
        let xml = r#"<?xml version="1.0"?>
        <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
            <url><loc>https://example.com/page1</loc></url>
            <url><loc>https://example.com/page2</loc></url>
        </urlset>"#;
        let urls = parse_sitemap_xml(xml).unwrap();
        assert_eq!(urls, vec![
            "https://example.com/page1",
            "https://example.com/page2"
        ]);
    }
}
```

**Step 4: Create collection command**

Create `src-tauri/src/commands/collection.rs`:
- `collect_from_sitemap(app: AppHandle, project_id: String)` — fetches sitemap, inserts URLs into DB with `source = 'sitemap'` and `indexed_status = 'unknown'`, emits progress events via `app.emit("collection_progress", ...)`

**Step 5: Run tests, build, commit**

```bash
cd src-tauri && cargo test
cargo build
git add -A
git commit -m "feat: add sitemap.xml parser with nested sitemap support"
```

---

## Task 6: CSV importer

**Files:**
- Modify: `src-tauri/Cargo.toml` (add csv, regex)
- Create: `src-tauri/src/crawler/csv_import.rs`
- Add command in `src-tauri/src/commands/collection.rs`

**Step 1: Add dependencies**

```toml
csv = "1.3"
regex = "1"
```

**Step 2: Write smart CSV importer**

Create `src-tauri/src/crawler/csv_import.rs`:
- `pub fn detect_url_columns(content: &str) -> Vec<(usize, String)>` — reads first 10 rows, applies URL regex (`https?://[^\s,\"]+`) to every cell, returns column indices that contain URLs along with the header name
- `pub fn extract_urls(content: &str, column_index: usize) -> Vec<String>` — extracts all URLs from the specified column
- `pub fn auto_extract_urls(content: &str) -> Vec<String>` — auto-detects the best URL column and extracts all URLs

**Step 3: Write tests**

```rust
#[test]
fn test_detect_urls_in_screaming_frog_csv() {
    let csv = "Address,Status Code,Title\nhttps://example.com/page1,200,Home\nhttps://example.com/page2,404,Not Found";
    let columns = detect_url_columns(csv);
    assert_eq!(columns[0].1, "Address");
}

#[test]
fn test_extract_urls() {
    let csv = "Address,Status Code\nhttps://example.com/a,200\nhttps://example.com/b,404";
    let urls = extract_urls(csv, 0);
    assert_eq!(urls.len(), 2);
}
```

**Step 4: Add Tauri command**

In `collection.rs`:
- `detect_csv_columns(content: String) -> Vec<CsvColumn>` — returns detected URL columns for UI selection
- `collect_from_csv(app: AppHandle, project_id: String, content: String, column_index: usize)` — imports URLs from CSV into DB

**Step 5: Test, build, commit**

```bash
cd src-tauri && cargo test
git add -A
git commit -m "feat: add smart CSV importer with auto URL detection"
```

---

## Task 7: Serper.dev API client

**Files:**
- Create: `src-tauri/src/crawler/serper.rs`
- Modify: `src-tauri/src/commands/collection.rs`

**Step 1: Write serper client**

Create `src-tauri/src/crawler/serper.rs`:
```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct SerperRequest {
    q: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    num: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    page: Option<u32>,
}

#[derive(Deserialize)]
pub struct SerperResponse {
    pub organic: Option<Vec<OrganicResult>>,
}

#[derive(Deserialize)]
pub struct OrganicResult {
    pub title: String,
    pub link: String,
    pub snippet: Option<String>,
}

pub async fn search_indexed_urls(
    api_key: &str,
    domain: &str,
    page: u32,
    num: u32,
) -> Result<SerperResponse, String> {
    let client = Client::new();
    let res = client
        .post("https://google.serper.dev/search")
        .header("X-API-KEY", api_key)
        .header("Content-Type", "application/json")
        .json(&SerperRequest {
            q: format!("site:{}", domain),
            num: Some(num),
            page: Some(page),
        })
        .send()
        .await
        .map_err(|e| e.to_string())?;
    res.json().await.map_err(|e| e.to_string())
}

pub async fn check_url_indexed(
    api_key: &str,
    domain: &str,
    path: &str,
) -> Result<bool, String> {
    let response = search_indexed_urls(
        api_key,
        &format!("{}{}", domain, path),
        1,
        10,
    ).await?;
    // If Google returns results containing our exact URL, it's indexed
    Ok(response.organic.map_or(false, |results| {
        results.iter().any(|r| r.link.contains(path))
    }))
}
```

**Step 2: Add collection command**

In `collection.rs`:
- `collect_from_serper(app: AppHandle, project_id: String)` — reads serper API key from settings, paginates through `site:domain` results, inserts URLs with `source = 'serper'` and `indexed_status = 'confirmed'`, emits progress events, stops when no more results

**Step 3: Build, commit**

```bash
cd src-tauri && cargo build
git add -A
git commit -m "feat: add serper.dev API client for indexed URL discovery"
```

---

## Task 8: Indexation verification

**Files:**
- Create: `src-tauri/src/commands/indexation.rs`
- Modify: `src-tauri/src/commands/mod.rs`

**Step 1: Write indexation verification command**

Create `src-tauri/src/commands/indexation.rs`:
- `get_unverified_urls(project_id: String) -> Vec<UrlEntry>` — returns URLs with `indexed_status = 'unknown'`
- `get_indexation_estimate(project_id: String) -> u64` — returns count of unverified URLs (= estimated serper credits)
- `verify_indexation(app: AppHandle, project_id: String)` — for each unverified URL, calls `check_url_indexed`, updates DB, emits progress. Uses a cancellation token (AtomicBool) to allow stopping.
- `stop_indexation(project_id: String)` — sets the cancellation token

**Step 2: Register commands, build, commit**

```bash
git add -A
git commit -m "feat: add indexation verification via serper.dev"
```

---

## Task 9: HTTP health checker (audit engine)

**Files:**
- Create: `src-tauri/src/crawler/checker.rs`
- Create: `src-tauri/src/commands/audit.rs`
- Modify: `src-tauri/src/commands/mod.rs`

**Step 1: Write HTTP checker**

Create `src-tauri/src/crawler/checker.rs`:
```rust
use reqwest::{Client, redirect::Policy};
use std::time::Instant;

pub struct CheckResult {
    pub url: String,
    pub http_status: i32,
    pub response_time_ms: i64,
    pub title: Option<String>,
    pub redirect_chain: Vec<String>,
    pub error: Option<String>,
}

pub async fn check_url(client: &Client, url: &str) -> CheckResult {
    let start = Instant::now();
    // Use a client with redirect policy that tracks hops
    // 1. First request with no auto-redirect to capture chain
    // 2. Follow redirects manually, recording each hop
    // 3. On final response: read status, parse <title> from HTML
    // 4. Record response time
    // ... implementation
}
```

Key implementation details:
- Build `reqwest::Client` with `Policy::none()` to manually follow redirects
- Loop: send request, if 3xx, record hop URL, follow Location header, max 10 hops
- On final response: record status code, response time
- If status is 200 and content-type is HTML, read body and extract `<title>` with a simple regex or string search (no heavy HTML parser needed)
- Detect redirect loops (same URL seen twice)

**Step 2: Write unit test**

```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_check_url_basic() {
        let client = reqwest::Client::new();
        let result = check_url(&client, "https://httpbin.org/status/200").await;
        assert_eq!(result.http_status, 200);
    }

    #[tokio::test]
    async fn test_check_url_404() {
        let client = reqwest::Client::new();
        let result = check_url(&client, "https://httpbin.org/status/404").await;
        assert_eq!(result.http_status, 404);
    }
}
```

**Step 3: Write audit command**

Create `src-tauri/src/commands/audit.rs`:
- `start_audit(app: AppHandle, project_id: String, concurrency: u32)` — queries DB for confirmed-indexed URLs not yet checked, uses `tokio::Semaphore` with given concurrency, checks each URL, saves result to DB immediately, emits `audit_progress` event after each URL
- `pause_audit(project_id: String)` — sets pause flag (AtomicBool)
- `resume_audit(app: AppHandle, project_id: String, concurrency: u32)` — clears pause, resumes
- `stop_audit(project_id: String)` — sets stop flag
- `update_concurrency(project_id: String, concurrency: u32)` — updates the semaphore permit count in real-time

Progress event payload:
```rust
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuditProgress {
    project_id: String,
    checked: u64,
    total: u64,
    current_url: String,
    last_result: Option<CheckResult>,
    stats: AuditStats,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuditStats {
    ok_count: u64,
    redirect_count: u64,
    not_found_count: u64,
    error_count: u64,
    empty_title_count: u64,
    slow_count: u64,
}
```

**Step 4: Test, build, commit**

```bash
cd src-tauri && cargo test
cargo build
git add -A
git commit -m "feat: add HTTP health checker with concurrent audit engine"
```

---

## Task 10: CSV export

**Files:**
- Create: `src-tauri/src/commands/export.rs`
- Modify: `src-tauri/src/commands/mod.rs`

**Step 1: Write export command**

Create `src-tauri/src/commands/export.rs`:
- `export_csv(project_id: String, filter: Option<String>) -> String` — queries URLs for project, optionally filtered (all/404/redirects/empty_title/slow), generates CSV string with columns: URL, Source, Indexed, HTTP Status, Response Time (ms), Title, Redirect Chain, Error
- Uses Tauri's dialog API to let user pick save location, or returns CSV content for frontend to handle

**Step 2: Add the `tauri-plugin-dialog` dependency**

```bash
cd src-tauri && cargo add tauri-plugin-dialog
```

In `lib.rs` add `.plugin(tauri_plugin_dialog::init())`.

Alternatively, use `tauri-plugin-fs` or just return the CSV string and let the frontend trigger a save dialog.

**Step 3: Build, commit**

```bash
git add -A
git commit -m "feat: add CSV export with filtering support"
```

---

## Task 11: Frontend — Layout, routing, and Settings page

**Files:**
- Install: `react-router-dom` (or use simple state-based routing)
- Create: `src/components/Layout.tsx`
- Create: `src/pages/Home.tsx` (placeholder)
- Create: `src/pages/Project.tsx` (placeholder)
- Create: `src/pages/Settings.tsx`
- Modify: `src/App.tsx`

**Step 1: Install router**

```bash
npm install react-router-dom
```

**Step 2: Create Layout component**

`src/components/Layout.tsx`:
- Dark background `#0a0a0f`
- Top bar with "Serp404" logo/text on left, settings gear icon on right
- `<Outlet />` for page content
- Style: Linear/Raycast inspired, subtle borders, clean typography

**Step 3: Create Settings page**

`src/pages/Settings.tsx`:
- serper.dev API key input (type password, with show/hide toggle)
- "Test Key" button that calls `invoke('test_serper_key', { apiKey })`
- Success/error feedback
- Save button that calls `invoke('set_setting', { key: 'serper_api_key', value })`
- Default timeout, user-agent, thread count fields

**Step 4: Wire up routing in App.tsx**

```tsx
import { BrowserRouter, Routes, Route } from 'react-router-dom';
// Note: For Tauri, use HashRouter instead of BrowserRouter
import Layout from './components/Layout';
import Home from './pages/Home';
import Project from './pages/Project';
import Settings from './pages/Settings';

function App() {
  return (
    <HashRouter>
      <Routes>
        <Route path="/" element={<Layout />}>
          <Route index element={<Home />} />
          <Route path="project/:id" element={<Project />} />
          <Route path="settings" element={<Settings />} />
        </Route>
      </Routes>
    </HashRouter>
  );
}
```

**Step 5: Verify, commit**

```bash
npm run tauri dev
# Verify: dark layout renders, settings page works, can save/load API key
git add -A
git commit -m "feat: add layout, routing, and settings page"
```

---

## Task 12: Frontend — Home page (project list)

**Files:**
- Create: `src/pages/Home.tsx`
- Create: `src/components/ProjectCard.tsx`
- Create: `src/components/NewProjectModal.tsx`

**Step 1: Build Home page**

`src/pages/Home.tsx`:
- "New Project" button (accent color, top right)
- Grid of `ProjectCard` components
- Calls `invoke('list_projects')` on mount
- Empty state: "No projects yet. Create one to get started."

**Step 2: Build ProjectCard**

`src/components/ProjectCard.tsx`:
- Shows: domain, created date, status badge
- Mini stats line: "X URLs | Y errors"
- Click navigates to `/project/:id`
- Delete button (with confirmation)

**Step 3: Build NewProjectModal**

`src/components/NewProjectModal.tsx`:
- Domain input field
- Source selection checkboxes:
  - Sitemap.xml (always available)
  - serper.dev (disabled with message if no API key)
  - CSV Upload with drag & drop zone
- "Create & Start Collection" button
- Calls `invoke('create_project', { domain })` then navigates to project page

**Step 4: Verify, commit**

```bash
npm run tauri dev
# Create a project, see it in the list, click into it
git add -A
git commit -m "feat: add home page with project list and creation modal"
```

---

## Task 13: Frontend — Project page (collection + indexation tabs)

**Files:**
- Create: `src/pages/Project.tsx` (full implementation)
- Create: `src/components/CollectionTab.tsx`
- Create: `src/components/IndexationTab.tsx`
- Create: `src/hooks/useTauriEvent.ts`

**Step 1: Create Tauri event hook**

`src/hooks/useTauriEvent.ts`:
```tsx
import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';

export function useTauriEvent<T>(event: string, handler: (payload: T) => void) {
  useEffect(() => {
    const unlisten = listen<T>(event, (e) => handler(e.payload));
    return () => { unlisten.then(fn => fn()); };
  }, [event, handler]);
}
```

**Step 2: Build Project page with tabs**

`src/pages/Project.tsx`:
- Loads project data via `invoke('get_project', { id })`
- 3 tabs: Collection | Indexation | Audit
- Tab bar styled like Linear (underline indicator)

**Step 3: Build CollectionTab**

`src/components/CollectionTab.tsx`:
- Buttons to trigger each source: "Fetch Sitemap" / "Search via serper" / "Upload CSV"
- CSV: file picker + column detection UI + confirm
- Progress indicators per source (listen to `collection_progress` event)
- Real-time URL counter
- Scrollable log of discovered URLs

**Step 4: Build IndexationTab**

`src/components/IndexationTab.tsx`:
- Shows count of unverified URLs
- "Estimated cost: X serper credits" display
- "Verify Indexation" button + Stop button
- Progress bar + live counter
- List of verified URLs with status badges (confirmed/not indexed)

**Step 5: Verify, commit**

```bash
npm run tauri dev
# Test: create project, fetch sitemap for a real domain, see URLs appear
git add -A
git commit -m "feat: add project page with collection and indexation tabs"
```

---

## Task 14: Frontend — Audit tab with real-time results

**Files:**
- Create: `src/components/AuditTab.tsx`
- Create: `src/components/ConcurrencySlider.tsx`
- Create: `src/components/ResultsTable.tsx`
- Create: `src/components/StatsBar.tsx`

**Step 1: Build ConcurrencySlider**

`src/components/ConcurrencySlider.tsx`:
- Range input: 1-50
- Shows current value
- Calls `invoke('update_concurrency', { projectId, concurrency })` on change

**Step 2: Build StatsBar**

`src/components/StatsBar.tsx`:
- Row of stat cards: OK (green) | Redirects (yellow) | 404 (red) | Errors (red) | Empty Titles (orange) | Slow (orange)
- Updates in real-time from audit progress events

**Step 3: Build ResultsTable**

`src/components/ResultsTable.tsx`:
- Columns: URL, Status, Response Time, Title, Redirects
- Color-coded status badges (green 200, yellow 301/302, red 404/500)
- Quick filter buttons: All | 404 | Redirections | Empty Titles | Slow (>2s)
- Sortable by any column
- Virtualized scrolling for large lists (use a simple approach: render visible rows only)

**Step 4: Build AuditTab**

`src/components/AuditTab.tsx`:
- ConcurrencySlider at top
- Start / Pause / Stop buttons
- Global progress bar (X / Y URLs checked)
- StatsBar
- ResultsTable
- "Export CSV" button
- Listens to `audit_progress` event for all updates

**Step 5: Verify end-to-end, commit**

```bash
npm run tauri dev
# Full test: create project → collect URLs → audit → see live results → export CSV
git add -A
git commit -m "feat: add audit tab with real-time results, stats, and CSV export"
```

---

## Task 15: Polish and README

**Files:**
- Modify: `README.md`
- Modify: UI components for polish (animations, transitions, hover states)

**Step 1: Polish UI**

- Add subtle transitions on tab switches
- Hover effects on cards and buttons
- Loading skeletons while data loads
- Error toasts/notifications
- Keyboard shortcuts (Escape to close modals)

**Step 2: Write README**

Update `README.md` with:
- Project description and screenshot placeholder
- Features list
- Prerequisites (Rust, Node.js)
- Installation instructions (`git clone`, `npm install`, `npm run tauri dev`)
- Build instructions (`npm run tauri build`)
- Usage guide (quick walkthrough)
- Contributing guidelines (brief)
- License (MIT)

**Step 3: Final commit**

```bash
git add -A
git commit -m "feat: polish UI and add comprehensive README"
```

---

## Dependency Summary

### Rust (src-tauri/Cargo.toml)
- `rusqlite` (0.32, bundled) — SQLite
- `reqwest` (0.12, json + rustls-tls) — HTTP client
- `tokio` (1, full) — Async runtime
- `serde` (1, derive) + `serde_json` (1) — Serialization
- `uuid` (1, v4) — ID generation
- `quick-xml` (0.37) — Sitemap XML parsing
- `csv` (1.3) — CSV import/export
- `regex` (1) — URL detection in CSV
- `tauri-plugin-dialog` — Save file dialog

### Frontend (package.json)
- `react-router-dom` — Routing
- `@tauri-apps/api` — Tauri IPC (included by default)

### Task Dependency Graph

```
Task 1 (scaffold)
  └── Task 2 (database)
        ├── Task 3 (project CRUD)
        ├── Task 4 (settings)
        │     └── Task 7 (serper client)
        │           └── Task 8 (indexation)
        ├── Task 5 (sitemap parser)
        ├── Task 6 (CSV importer)
        └── Task 9 (HTTP checker)
              └── Task 10 (CSV export)

Task 1 (scaffold)
  └── Task 11 (layout + settings UI)
        └── Task 12 (home page)
              └── Task 13 (collection + indexation tabs)
                    └── Task 14 (audit tab)
                          └── Task 15 (polish + README)
```

Backend tasks (2-10) and frontend tasks (11-14) can be parallelized after Task 1.
