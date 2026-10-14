import { describe, expect, it } from "vitest";
import { fileEntry } from "@/test/factories";
import { mountWithPlugins } from "@/test/mount";
import FileTable from "./FileTable.vue";
import PaginationBar from "./PaginationBar.vue";
import PermissionBanner from "./PermissionBanner.vue";
import ScanProgress from "./ScanProgress.vue";
import SizeBar from "./SizeBar.vue";

describe("SizeBar", () => {
  it("scales relative to the max with a visible minimum", async () => {
    const width = async (value: number, max: number) => {
      const { wrapper } = await mountWithPlugins(SizeBar, { props: { value, max } });
      return (wrapper.find(".fill").element as HTMLElement).style.width;
    };
    expect(await width(50, 100)).toBe("50%");
    expect(await width(1, 100_000)).toBe("1%");
    expect(await width(0, 100)).toBe("0%");
    expect(await width(10, 0)).toBe("0%");
  });
});

describe("FileTable", () => {
  const items = [
    fileEntry({ id: 1, path: "/Users/alice/Movies/big.mov", sizeOnDisk: 5_000_000_000 }),
    fileEntry({
      id: 2,
      path: "/Users/alice/Library/Caches",
      name: "Caches",
      kind: "Directory",
      sizeOnDisk: 2_000_000_000,
      fileCount: 1234,
      isProtected: true,
    }),
  ];

  it("renders name, shortened location, size and item count", async () => {
    const { wrapper } = await mountWithPlugins(FileTable, {
      props: { items, maxSize: 5_000_000_000, home: "/Users/alice" },
    });
    const first = wrapper.find('[data-test="row-1"]');
    expect(first.text()).toContain("big.mov");
    expect(first.text()).toContain("~/Movies");
    expect(first.text()).toContain("5,0 GB");
    const second = wrapper.find('[data-test="row-2"]');
    expect(second.text()).toContain("1.234");
    expect(second.find(".badge").exists()).toBe(true);
  });

  it("only offers drill-down for folders and emits actions with the path", async () => {
    const { wrapper } = await mountWithPlugins(FileTable, {
      props: { items, maxSize: 1, home: null },
    });
    expect(wrapper.find('[data-test="row-1"] [data-test="open-folder"]').exists()).toBe(false);

    await wrapper.find('[data-test="row-2"] [data-test="open-folder"]').trigger("click");
    await wrapper.find('[data-test="row-1"] [data-test="reveal"]').trigger("click");

    expect(wrapper.emitted("openFolder")).toEqual([["/Users/alice/Library/Caches"]]);
    expect(wrapper.emitted("reveal")).toEqual([["/Users/alice/Movies/big.mov"]]);
  });
});

describe("PaginationBar", () => {
  it("shows the visible range and disables unavailable directions", async () => {
    const { wrapper } = await mountWithPlugins(PaginationBar, {
      props: { offset: 100, count: 100, total: 250, hasPrevious: true, hasNext: false },
    });
    expect(wrapper.find('[data-test="range"]').text()).toBe("101–200 de 250");
    expect(wrapper.find('[data-test="next"]').attributes("disabled")).toBeDefined();
    await wrapper.find('[data-test="previous"]').trigger("click");
    expect(wrapper.emitted("previous")).toHaveLength(1);
  });

  it("shows zero range for empty results", async () => {
    const { wrapper } = await mountWithPlugins(PaginationBar, {
      props: { offset: 0, count: 0, total: 0, hasPrevious: false, hasNext: false },
    });
    expect(wrapper.find('[data-test="range"]').text()).toBe("0–0 de 0");
  });
});

describe("ScanProgress", () => {
  const progress = {
    filesSeen: 12345,
    bytesSeen: 3_500_000_000,
    inaccessibleCount: 2,
    currentPath: "/x/y",
  };

  it("shows live counters and a cancel button while running", async () => {
    const { wrapper } = await mountWithPlugins(ScanProgress, {
      props: { progress, running: true },
    });
    expect(wrapper.find('[data-test="files-seen"]').text()).toBe("12.345");
    expect(wrapper.find('[data-test="bytes-seen"]').text()).toBe("3,5 GB");
    expect(wrapper.text()).toContain("/x/y");
    await wrapper.find('[data-test="cancel"]').trigger("click");
    expect(wrapper.emitted("cancel")).toHaveLength(1);
  });

  it("hides cancel and current path when finished", async () => {
    const { wrapper } = await mountWithPlugins(ScanProgress, {
      props: { progress, running: false },
    });
    expect(wrapper.find('[data-test="cancel"]').exists()).toBe(false);
    expect(wrapper.text()).not.toContain("/x/y");
  });
});

describe("PermissionBanner", () => {
  it("emits open and recheck", async () => {
    const { wrapper } = await mountWithPlugins(PermissionBanner);
    const [open, recheck] = wrapper.findAll("button");
    await open?.trigger("click");
    await recheck?.trigger("click");
    expect(wrapper.emitted("open")).toHaveLength(1);
    expect(wrapper.emitted("recheck")).toHaveLength(1);
  });
});
