describe("Serp404 E2E", () => {
  describe("App launch", () => {
    it("should display the Serp404 title in the header", async () => {
      const header = await $("text=Serp404");
      await expect(header).toBeExisting();
    });

    it("should show the Projects heading on home page", async () => {
      const heading = await $("h1=Projects");
      await expect(heading).toBeExisting();
    });

    it("should show empty state when no projects exist", async () => {
      const emptyText = await $("*=No projects yet");
      await expect(emptyText).toBeExisting();
    });
  });

  describe("Project creation", () => {
    it("should open new project modal", async () => {
      const newBtn = await $("button*=New Project");
      await newBtn.click();

      const modal = await $("*=New Project");
      await expect(modal).toBeExisting();
    });

    it("should create a project with a domain", async () => {
      const input = await $("input[placeholder*='example']");
      await input.setValue("test-domain.com");

      const createBtn = await $("button*=Create");
      await createBtn.click();

      // Should navigate to project page
      const projectTitle = await $("*=test-domain.com");
      await expect(projectTitle).toBeExisting();
    });
  });

  describe("Settings", () => {
    it("should navigate to settings page", async () => {
      const settingsLink = await $("a[href*='settings']");
      await settingsLink.click();

      const heading = await $("h1=Settings");
      await expect(heading).toBeExisting();
    });

    it("should have API key input field", async () => {
      const input = await $("input[type='password']");
      await expect(input).toBeExisting();
    });

    it("should save and load API key", async () => {
      const input = await $("input[type='password']");
      await input.setValue("test-api-key-123");

      const saveBtn = await $("button*=Save");
      await saveBtn.click();

      // Reload and verify key is still there
      await browser.refresh();
      const reloadedInput = await $("input[type='password']");
      const value = await reloadedInput.getValue();
      expect(value).toBe("test-api-key-123");
    });
  });

  describe("Project workflow", () => {
    before(async () => {
      // Navigate to home and create a test project
      await browser.url("#/");
      const newBtn = await $("button*=New Project");
      await newBtn.click();
      const input = await $("input[placeholder*='example']");
      await input.setValue("e2e-test.com");
      const createBtn = await $("button*=Create");
      await createBtn.click();
      await browser.pause(1000);
    });

    it("should show collection tab by default", async () => {
      const tab = await $("*=Collection");
      await expect(tab).toBeExisting();
    });

    it("should have fetch sitemap button", async () => {
      const btn = await $("button*=Fetch Sitemap");
      await expect(btn).toBeExisting();
    });

    it("should have upload CSV button", async () => {
      const btn = await $("button*=Upload CSV");
      await expect(btn).toBeExisting();
    });

    it("should switch to indexation tab", async () => {
      const tab = await $("*=Indexation");
      await tab.click();

      const content = await $("*=indexation");
      await expect(content).toBeExisting();
    });

    it("should switch to audit tab", async () => {
      const tab = await $("*=Audit");
      await tab.click();

      // Should show the concurrency slider
      const slider = await $("input[type='range']");
      await expect(slider).toBeExisting();
    });

    it("should have export CSV button in audit tab", async () => {
      const btn = await $("button*=Export");
      await expect(btn).toBeExisting();
    });
  });
});
