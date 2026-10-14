import { describe, expect, it, vi } from "vitest";
import { analysis, fileEntry } from "@/test/factories";
import { mountWithPlugins } from "@/test/mount";
import AnalysisPanel from "./AnalysisPanel.vue";
import RiskBadge from "./RiskBadge.vue";

describe("RiskBadge", () => {
  it("shows the translated level", async () => {
    const { wrapper } = await mountWithPlugins(RiskBadge, { props: { level: "Dangerous" } });
    expect(wrapper.text()).toBe("Perigoso");
    expect(wrapper.classes()).toContain("dangerous");
  });
});

describe("AnalysisPanel", () => {
  const item = fileEntry({
    id: 1,
    name: "CoreSimulator",
    path: "/Users/alice/Library/Developer/CoreSimulator",
    kind: "Directory",
  });
  const props = {
    item,
    analysis: analysis(),
    analyzing: false,
    aiEnabled: true,
    home: "/Users/alice",
    error: null,
  };

  it("shows level, source, recommendation, steps and copyable commands", async () => {
    const writeText = vi.fn(async () => undefined);
    Object.assign(navigator, { clipboard: { writeText } });
    const { wrapper } = await mountWithPlugins(AnalysisPanel, { props });

    expect(wrapper.text()).toContain("~/Library/Developer/CoreSimulator");
    expect(wrapper.text()).toContain("Atenção");
    expect(wrapper.find('[data-test="source"]').text()).toBe("IA · anthropic · claude-opus-5-5");
    expect(wrapper.find('[data-test="recommendation"]').text()).toBe(
      "Limpe pelo app ou por comando oficial",
    );
    expect(wrapper.text()).toContain("Xcode › Settings › Platforms");
    expect(wrapper.find('[data-test="commands"]').text()).toContain(
      "xcrun simctl delete unavailable",
    );
    expect(wrapper.find('[data-test="configure-ai"]').exists()).toBe(false);

    const copy = wrapper.findAll("button").find((b) => b.text() === "Copiar");
    await copy?.trigger("click");
    expect(writeText).toHaveBeenCalledWith("xcrun simctl delete unavailable");
    await vi.waitFor(() => expect(wrapper.text()).toContain("Copiado!"));
  });

  it("explains a fallback to local rules and suggests configuring AI", async () => {
    const { wrapper } = await mountWithPlugins(AnalysisPanel, {
      props: {
        ...props,
        aiEnabled: false,
        analysis: analysis({
          source: { kind: "heuristic" },
          aiError: "rate limited",
          commands: [],
        }),
      },
    });
    expect(wrapper.find('[data-test="source"]').text()).toBe("Regras locais");
    expect(wrapper.find('[data-test="ai-error"]').text()).toContain("rate limited");
    expect(wrapper.find('[data-test="commands"]').exists()).toBe(false);
    expect(wrapper.find('[data-test="configure-ai"]').exists()).toBe(true);
  });

  it("shows progress and emits actions", async () => {
    const { wrapper } = await mountWithPlugins(AnalysisPanel, {
      props: { ...props, analysis: null, analyzing: true, error: "boom" },
    });
    expect(wrapper.find('[data-test="analyzing"]').exists()).toBe(true);
    expect(wrapper.text()).toContain("boom");
    expect(wrapper.find('[data-test="reanalyze"]').attributes("disabled")).toBeDefined();

    await wrapper.find('[data-test="close"]').trigger("click");
    const reveal = wrapper.findAll("button").find((b) => b.text() === "Mostrar no Finder");
    await reveal?.trigger("click");
    expect(wrapper.emitted("close")).toHaveLength(1);
    expect(wrapper.emitted("reveal")).toEqual([[item.path]]);
  });

  it("closes when clicking the backdrop", async () => {
    const { wrapper } = await mountWithPlugins(AnalysisPanel, { props });
    await wrapper.find(".backdrop").trigger("click");
    expect(wrapper.emitted("close")).toHaveLength(1);
  });
});
