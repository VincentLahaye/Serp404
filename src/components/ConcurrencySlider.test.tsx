import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import ConcurrencySlider from "./ConcurrencySlider";

describe("ConcurrencySlider", () => {
  it("renders with initial value and plural label", () => {
    render(<ConcurrencySlider value={10} onChange={() => {}} />);
    expect(screen.getByText("10 threads")).toBeInTheDocument();
  });

  it("renders singular label for value of 1", () => {
    render(<ConcurrencySlider value={1} onChange={() => {}} />);
    expect(screen.getByText("1 thread")).toBeInTheDocument();
  });

  it("can be disabled", () => {
    render(<ConcurrencySlider value={5} onChange={() => {}} disabled />);
    const slider = screen.getByRole("slider");
    expect(slider).toBeDisabled();
  });

  it("is enabled by default", () => {
    render(<ConcurrencySlider value={5} onChange={() => {}} />);
    const slider = screen.getByRole("slider");
    expect(slider).not.toBeDisabled();
  });

  it("slider has correct min, max, and step attributes", () => {
    render(<ConcurrencySlider value={25} onChange={() => {}} />);
    const slider = screen.getByRole("slider") as HTMLInputElement;
    expect(slider.min).toBe("1");
    expect(slider.max).toBe("50");
    expect(slider.step).toBe("1");
    expect(slider.value).toBe("25");
  });
});
