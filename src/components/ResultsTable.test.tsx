import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import ResultsTable from "./ResultsTable";

const mockResults = [
  {
    url: "https://example.com/ok",
    httpStatus: 200,
    responseTimeMs: 150,
    title: "OK Page",
    redirectChain: null,
    error: null,
  },
  {
    url: "https://example.com/not-found",
    httpStatus: 404,
    responseTimeMs: 100,
    title: null,
    redirectChain: null,
    error: null,
  },
  {
    url: "https://example.com/slow",
    httpStatus: 200,
    responseTimeMs: 3000,
    title: "Slow Page",
    redirectChain: null,
    error: null,
  },
  {
    url: "https://example.com/redirect",
    httpStatus: 301,
    responseTimeMs: 200,
    title: "Redirected",
    redirectChain: "https://example.com/old -> https://example.com/new",
    error: null,
  },
];

describe("ResultsTable", () => {
  it("renders all results", () => {
    render(
      <ResultsTable results={mockResults} filter="all" onFilterChange={() => {}} />
    );
    expect(screen.getByText("https://example.com/ok")).toBeInTheDocument();
    expect(
      screen.getByText("https://example.com/not-found")
    ).toBeInTheDocument();
    expect(screen.getByText("https://example.com/slow")).toBeInTheDocument();
    expect(
      screen.getByText("https://example.com/redirect")
    ).toBeInTheDocument();
  });

  it("shows filter buttons", () => {
    render(
      <ResultsTable results={mockResults} filter="all" onFilterChange={() => {}} />
    );
    expect(screen.getByText("All")).toBeInTheDocument();
    // "Redirects" appears in both the filter button and the table header,
    // so use getAllByText to verify at least one is present.
    expect(screen.getAllByText("Redirects").length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText("Empty Titles")).toBeInTheDocument();
    expect(screen.getByText("Slow")).toBeInTheDocument();
  });

  it("displays status badges with correct text", () => {
    render(
      <ResultsTable results={mockResults} filter="all" onFilterChange={() => {}} />
    );
    // 200 badge for the two 200-status results
    const badges200 = screen.getAllByText("200");
    expect(badges200.length).toBeGreaterThanOrEqual(2);

    // 404 badge
    // The filter button also contains "404" so we look for badge text
    const badges404 = screen.getAllByText("404");
    expect(badges404.length).toBeGreaterThanOrEqual(1);

    // 301 badge
    expect(screen.getByText("301")).toBeInTheDocument();
  });

  it("renders table column headers", () => {
    render(
      <ResultsTable results={mockResults} filter="all" onFilterChange={() => {}} />
    );
    expect(screen.getByText("URL")).toBeInTheDocument();
    expect(screen.getByText("Status")).toBeInTheDocument();
    expect(screen.getByText("Time (ms)")).toBeInTheDocument();
    expect(screen.getByText("Title")).toBeInTheDocument();
  });

  it("calls onFilterChange when a filter button is clicked", async () => {
    const user = userEvent.setup();
    const onFilterChange = vi.fn();
    render(
      <ResultsTable
        results={mockResults}
        filter="all"
        onFilterChange={onFilterChange}
      />
    );

    await user.click(screen.getByText("Slow"));
    expect(onFilterChange).toHaveBeenCalledWith("slow");
  });

  it("shows empty state when no results", () => {
    render(
      <ResultsTable results={[]} filter="all" onFilterChange={() => {}} />
    );
    expect(
      screen.getByText("No results yet. Start an audit to see data here.")
    ).toBeInTheDocument();
  });

  it("shows 'no match' message when filter yields no results", () => {
    // Only a 200 result, filter on 404
    const results = [
      {
        url: "https://example.com/ok",
        httpStatus: 200,
        responseTimeMs: 100,
        title: "Page",
        redirectChain: null,
        error: null,
      },
    ];
    render(
      <ResultsTable results={results} filter="404" onFilterChange={() => {}} />
    );
    expect(
      screen.getByText("No results match the current filter.")
    ).toBeInTheDocument();
  });

  it("displays response times", () => {
    render(
      <ResultsTable results={mockResults} filter="all" onFilterChange={() => {}} />
    );
    expect(screen.getByText("150")).toBeInTheDocument();
    expect(screen.getByText("3000")).toBeInTheDocument();
  });
});
