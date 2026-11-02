import { expect, test } from "./helpers";

// Authored camera shots (#253) are played by @theatre/core, vendored as one ESM file behind the importmap.
test("@theatre/core imports from the game's own vendored file", async ({ page }) => {
  await page.goto("/");
  const api = await page.evaluate(async () => {
    const core: any = await import("@theatre/core" as string);
    const project = core.getProject("sixfold-import-check");
    return { getProject: typeof core.getProject, name: project.address.projectId };
  });
  expect(api).toEqual({ getProject: "function", name: "sixfold-import-check" });
});
