# E2E Tests

End-to-end tests using WebdriverIO with Tauri WebDriver.

## Prerequisites

1. Build the app first: `npm run tauri build`
2. The built binary must exist at `src-tauri/target/release/serp404`

## Running

```bash
npm run test:e2e
```

## Writing Tests

Tests are in `e2e/*.test.ts`. They use WebdriverIO browser commands to interact with the app's UI.

See [Tauri E2E Testing docs](https://v2.tauri.app/develop/tests/) for more information.
