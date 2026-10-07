import { describe, expect, it } from "vitest";

import type { Drive } from "../model/drives";
import {
  activeTitle,
  chooseDrive,
  describeDrive,
  formatBytes,
  loadActive,
  loadDrives,
  searchPlaceholder,
} from "./drivePresenter";

const GB = 1024 ** 3;

function drive(letter: string, more: Partial<Drive> = {}): Drive {
  return {
    letter,
    label: "",
    filesystem: "NTFS",
    removable: false,
    total: 512 * GB,
    free: 128 * GB,
    method: "mft",
    ...more,
  };
}

describe("loadActive", () => {
  it("reads the active drive, or null when it cannot be read", async () => {
    expect(await loadActive(async () => "E")).toBe("E");
    expect(await loadActive(async () => Promise.reject("unreadable"))).toBeNull();
  });
});

describe("loadDrives", () => {
  it("lists the drives", async () => {
    const drives = [drive("C"), drive("E")];
    expect(await loadDrives(async () => drives)).toEqual({ drives, error: null });
  });

  it("reports a failed listing", async () => {
    expect(await loadDrives(async () => Promise.reject("busy"))).toEqual({
      drives: [],
      error: "busy",
    });
  });
});

describe("chooseDrive", () => {
  it("saves the new drive and makes it active", async () => {
    const saved: string[] = [];
    const result = await chooseDrive("C", "E", async (letter) => {
      saved.push(letter);
      return letter;
    });
    expect(saved).toEqual(["E"]);
    expect(result).toEqual({ active: "E", error: null });
  });

  it("does nothing when the drive is already active", async () => {
    let calls = 0;
    const result = await chooseDrive("C", "C", async (letter) => {
      calls += 1;
      return letter;
    });
    expect(calls).toBe(0);
    expect(result).toEqual({ active: "C", error: null });
  });

  it("keeps the old drive when switching fails", async () => {
    const result = await chooseDrive("C", "E", async () => Promise.reject("locked"));
    expect(result).toEqual({ active: "C", error: "locked" });
  });
});

describe("wording", () => {
  it("names the active drive like Explorer, or by letter when it is not listed", () => {
    const drives = [drive("C", { label: "Windows" })];
    expect(activeTitle(drives, "C")).toBe("Windows (C:)");
    expect(activeTitle(drives, "F")).toBe("F:");
    expect(activeTitle(drives, null)).toBe("Choose a drive");
  });

  it("says which drive the search covers", () => {
    expect(searchPlaceholder("C")).toBe("Search C:");
    expect(searchPlaceholder(null)).toBe("Search files and folders");
    expect(searchPlaceholder("E", "contents")).toBe("Search names and contents on E:");
    expect(searchPlaceholder(null, "contents")).toBe("Search names and contents");
  });

  it("words size, label and method like Explorer", () => {
    expect(describeDrive(drive("C", { label: "Windows" }))).toEqual({
      title: "Windows (C:)",
      detail: "128 GB free of 512 GB",
      method: "NTFS · instant, stays current",
      usedPercent: 75,
    });
    expect(
      describeDrive(drive("E", { removable: true, filesystem: "exFAT", method: "walk" })).title,
    ).toBe("USB Drive (E:)");
  });

  it("formats bytes in powers of 1024", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(1.8 * 1024 ** 4)).toBe("1.8 TB");
    expect(formatBytes(512 * GB)).toBe("512 GB");
  });
});
