import { describe, it, expect, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import NewProjectModal from "./NewProjectModal";
import { mockInvoke, resetInvokeMock, mockedInvoke } from "../test/tauri-mock";

function renderModal(onClose = () => {}) {
  return render(
    <MemoryRouter>
      <NewProjectModal onClose={onClose} />
    </MemoryRouter>
  );
}

describe("NewProjectModal", () => {
  beforeEach(() => resetInvokeMock());

  it("renders the modal with heading and form", () => {
    renderModal();
    expect(screen.getByText("New Project")).toBeInTheDocument();
    expect(screen.getByText("Domain")).toBeInTheDocument();
    expect(screen.getByPlaceholderText("example.com")).toBeInTheDocument();
    expect(screen.getByText("Create Project")).toBeInTheDocument();
  });

  it("shows error when submitting empty domain", async () => {
    const user = userEvent.setup();
    renderModal();

    const submitBtn = screen.getByText("Create Project");
    await user.click(submitBtn);

    await waitFor(() => {
      expect(screen.getByText("Please enter a domain.")).toBeInTheDocument();
    });
  });

  it("strips protocol from domain input on submit", async () => {
    const user = userEvent.setup();
    mockInvoke({
      create_project: {
        id: "1",
        domain: "example.com",
        status: "created",
        createdAt: "",
        updatedAt: "",
      },
    });
    renderModal();

    const input = screen.getByPlaceholderText("example.com");
    await user.type(input, "https://example.com");

    const submitBtn = screen.getByText("Create Project");
    await user.click(submitBtn);

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("create_project", {
        domain: "example.com",
      });
    });
  });

  it("strips trailing slashes from domain", async () => {
    const user = userEvent.setup();
    mockInvoke({
      create_project: {
        id: "2",
        domain: "example.com",
        status: "created",
        createdAt: "",
        updatedAt: "",
      },
    });
    renderModal();

    const input = screen.getByPlaceholderText("example.com");
    await user.type(input, "http://example.com///");

    const submitBtn = screen.getByText("Create Project");
    await user.click(submitBtn);

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("create_project", {
        domain: "example.com",
      });
    });
  });

  it("calls onClose when close button is clicked", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    renderModal(onClose);

    const closeBtn = screen.getByLabelText("Close");
    await user.click(closeBtn);

    expect(onClose).toHaveBeenCalled();
  });

  it("calls onClose after successful project creation", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    mockInvoke({
      create_project: {
        id: "1",
        domain: "test.com",
        status: "created",
        createdAt: "",
        updatedAt: "",
      },
    });

    render(
      <MemoryRouter>
        <NewProjectModal onClose={onClose} />
      </MemoryRouter>
    );

    const input = screen.getByPlaceholderText("example.com");
    await user.type(input, "test.com");

    const submitBtn = screen.getByText("Create Project");
    await user.click(submitBtn);

    await waitFor(() => {
      expect(onClose).toHaveBeenCalled();
    });
  });
});
