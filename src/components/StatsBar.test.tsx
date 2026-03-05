import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import StatsBar from "./StatsBar";

describe("StatsBar", () => {
  it("renders all stat category labels", () => {
    render(
      <StatsBar
        stats={{
          okCount: 10,
          redirectCount: 5,
          notFoundCount: 3,
          errorCount: 2,
          emptyTitleCount: 1,
          slowCount: 4,
        }}
      />
    );

    expect(screen.getByText("OK")).toBeInTheDocument();
    expect(screen.getByText("Redirects")).toBeInTheDocument();
    expect(screen.getByText("404")).toBeInTheDocument();
    expect(screen.getByText("Errors")).toBeInTheDocument();
    expect(screen.getByText("Empty Titles")).toBeInTheDocument();
    expect(screen.getByText("Slow (>2s)")).toBeInTheDocument();
  });

  it("renders correct stat values", () => {
    render(
      <StatsBar
        stats={{
          okCount: 10,
          redirectCount: 5,
          notFoundCount: 3,
          errorCount: 2,
          emptyTitleCount: 1,
          slowCount: 4,
        }}
      />
    );

    expect(screen.getByText("10")).toBeInTheDocument();
    expect(screen.getByText("5")).toBeInTheDocument();
    expect(screen.getByText("3")).toBeInTheDocument();
    expect(screen.getByText("2")).toBeInTheDocument();
    expect(screen.getByText("1")).toBeInTheDocument();
    expect(screen.getByText("4")).toBeInTheDocument();
  });

  it("renders zero values", () => {
    render(
      <StatsBar
        stats={{
          okCount: 0,
          redirectCount: 0,
          notFoundCount: 0,
          errorCount: 0,
          emptyTitleCount: 0,
          slowCount: 0,
        }}
      />
    );

    // All six stat cards should show 0
    const zeros = screen.getAllByText("0");
    expect(zeros).toHaveLength(6);
  });
});
