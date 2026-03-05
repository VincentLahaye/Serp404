import type { Options } from "@wdio/types";
import { join } from "path";

export const config: Options.Testrunner = {
  runner: "local",
  specs: ["./e2e/**/*.test.ts"],
  maxInstances: 1,
  capabilities: [
    {
      "tauri:options": {
        application: join(
          process.cwd(),
          "src-tauri/target/release/serp404"
        ),
      },
    } as any,
  ],
  framework: "mocha",
  reporters: ["spec"],
  mochaOpts: {
    ui: "bdd",
    timeout: 60000,
  },
};
