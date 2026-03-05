import { describe, it, expect, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import Settings from "./Settings";
import { mockInvoke, resetInvokeMock, mockedInvoke } from "../test/tauri-mock";

function renderSettings() {
  return render(
    <MemoryRouter>
      <Settings />
    </MemoryRouter>
  );
}

describe("Settings", () => {
  beforeEach(() => resetInvokeMock());

  it("loads existing API key on mount", async () => {
    mockInvoke({ get_setting: "sk-test-key-123" });
    renderSettings();

    await waitFor(() => {
      const input = screen.getByPlaceholderText(
        "Enter your serper.dev API key"
      ) as HTMLInputElement;
      expect(input.value).toBe("sk-test-key-123");
    });
  });

  it("shows password field by default", async () => {
    mockInvoke({ get_setting: null });
    renderSettings();

    await waitFor(() => {
      const input = screen.getByPlaceholderText(
        "Enter your serper.dev API key"
      ) as HTMLInputElement;
      expect(input.type).toBe("password");
    });
  });

  it("toggles password visibility", async () => {
    const user = userEvent.setup();
    mockInvoke({ get_setting: null });
    renderSettings();

    const input = screen.getByPlaceholderText(
      "Enter your serper.dev API key"
    ) as HTMLInputElement;
    expect(input.type).toBe("password");

    // Click the "Show API key" toggle button
    const toggleBtn = screen.getByLabelText("Show API key");
    await user.click(toggleBtn);
    expect(input.type).toBe("text");

    // Click again to hide
    const hideBtn = screen.getByLabelText("Hide API key");
    await user.click(hideBtn);
    expect(input.type).toBe("password");
  });

  it("saves API key on save click", async () => {
    const user = userEvent.setup();
    mockInvoke({
      get_setting: null,
      set_setting: undefined,
    });
    renderSettings();

    const input = screen.getByPlaceholderText("Enter your serper.dev API key");
    await user.type(input, "my-new-key");

    const saveBtn = screen.getByText("Save");
    await user.click(saveBtn);

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("set_setting", {
        key: "serper_api_key",
        value: "my-new-key",
      });
    });

    // Should show "Saved" confirmation
    await waitFor(() => {
      expect(screen.getByText("Saved")).toBeInTheDocument();
    });
  });

  it("tests API key validation — success", async () => {
    const user = userEvent.setup();
    mockInvoke({
      get_setting: null,
      test_serper_key: true,
    });
    renderSettings();

    const input = screen.getByPlaceholderText("Enter your serper.dev API key");
    await user.type(input, "valid-key");

    const testBtn = screen.getByText("Test Key");
    await user.click(testBtn);

    await waitFor(() => {
      expect(screen.getByText("Key is valid")).toBeInTheDocument();
    });
  });

  it("tests API key validation — failure", async () => {
    const user = userEvent.setup();
    mockInvoke({
      get_setting: null,
      test_serper_key: false,
    });
    renderSettings();

    const input = screen.getByPlaceholderText("Enter your serper.dev API key");
    await user.type(input, "bad-key");

    const testBtn = screen.getByText("Test Key");
    await user.click(testBtn);

    await waitFor(() => {
      expect(
        screen.getByText("Invalid key or request failed")
      ).toBeInTheDocument();
    });
  });

  it("disables Test Key and Save buttons when input is empty", () => {
    mockInvoke({ get_setting: null });
    renderSettings();

    const testBtn = screen.getByText("Test Key");
    const saveBtn = screen.getByText("Save");

    expect(testBtn).toBeDisabled();
    expect(saveBtn).toBeDisabled();
  });
});
