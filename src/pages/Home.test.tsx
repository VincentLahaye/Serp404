import { describe, it, expect, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import Home from "./Home";
import { mockInvoke, resetInvokeMock } from "../test/tauri-mock";

function renderHome() {
  return render(
    <MemoryRouter>
      <Home />
    </MemoryRouter>
  );
}

describe("Home", () => {
  beforeEach(() => resetInvokeMock());

  it("shows empty state when no projects", async () => {
    mockInvoke({ list_projects: [] });
    renderHome();

    await waitFor(() => {
      expect(
        screen.getByText("No projects yet. Create one to get started.")
      ).toBeInTheDocument();
    });
  });

  it("renders project cards when projects exist", async () => {
    mockInvoke({
      list_projects: [
        {
          id: "1",
          domain: "example.com",
          status: "created",
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
      ],
      get_project_stats: {
        totalUrls: 10,
        confirmedIndexed: 5,
        notIndexed: 2,
        unknownStatus: 3,
        checked: 4,
        okCount: 3,
        redirectCount: 1,
        notFoundCount: 0,
        errorCount: 0,
        emptyTitleCount: 0,
        slowCount: 0,
      },
    });
    renderHome();

    await waitFor(() => {
      expect(screen.getByText("example.com")).toBeInTheDocument();
    });
  });

  it("shows new project button", async () => {
    mockInvoke({ list_projects: [] });
    renderHome();

    await waitFor(() => {
      expect(screen.getByText("+ New Project")).toBeInTheDocument();
    });
  });

  it("shows Create Project button in empty state", async () => {
    mockInvoke({ list_projects: [] });
    renderHome();

    await waitFor(() => {
      expect(screen.getByText("Create Project")).toBeInTheDocument();
    });
  });

  it("renders multiple project cards", async () => {
    mockInvoke({
      list_projects: [
        {
          id: "1",
          domain: "alpha.com",
          status: "created",
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
        {
          id: "2",
          domain: "beta.com",
          status: "done",
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
      ],
      get_project_stats: {
        totalUrls: 0,
        confirmedIndexed: 0,
        notIndexed: 0,
        unknownStatus: 0,
        checked: 0,
        okCount: 0,
        redirectCount: 0,
        notFoundCount: 0,
        errorCount: 0,
        emptyTitleCount: 0,
        slowCount: 0,
      },
    });
    renderHome();

    await waitFor(() => {
      expect(screen.getByText("alpha.com")).toBeInTheDocument();
      expect(screen.getByText("beta.com")).toBeInTheDocument();
    });
  });
});
