import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import test, { type TestContext } from "node:test";
import { build } from "esbuild";
import type { ComponentProps } from "react";

import type { DictionaryCatalogEntry, DictionaryImportProgress, DictionarySource } from "../src/dictionary/types.ts";

type SectionProps = ComponentProps<typeof import("../src/settings/sections/DictionarySettingsSection.tsx").DictionarySettingsSection>;

// Exercise the real API and settings component without starting a browser or app.
const bundle = await build({
  stdin: {
    contents: `
      import { createElement } from "react";
      import { renderToStaticMarkup } from "react-dom/server";
      import { createInstance } from "i18next";
      import { I18nextProvider } from "react-i18next";
      import zh from "./src/i18n/locales/zh-CN.json";
      import { DictionarySettingsSection } from "./src/settings/sections/DictionarySettingsSection.tsx";
      export { dictionaryApi } from "./src/dictionary/api.ts";
      export async function renderDictionary(props) {
        const i18n = createInstance();
        await i18n.init({ lng: "zh-CN", resources: { "zh-CN": zh }, interpolation: { escapeValue: false } });
        return renderToStaticMarkup(createElement(I18nextProvider, { i18n }, createElement(DictionarySettingsSection, props)));
      }
    `,
    resolveDir: fileURLToPath(new URL("..", import.meta.url)),
  },
  bundle: true,
  write: false,
  format: "esm",
  platform: "node",
  jsx: "automatic",
  banner: { js: `import { createRequire } from "node:module"; const require = createRequire(${JSON.stringify(fileURLToPath(new URL("../package.json", import.meta.url)))});` },
  define: { "import.meta.env.VITE_VRCS_SESSION_TOKEN": '""' },
});
const { dictionaryApi, renderDictionary } = await import(
  `data:text/javascript;base64,${Buffer.from(bundle.outputFiles[0].text).toString("base64")}`
) as {
  dictionaryApi: typeof import("../src/dictionary/api.ts").dictionaryApi;
  renderDictionary: (props: SectionProps) => Promise<string>;
};

const catalog: DictionaryCatalogEntry[] = ["zh", "ja"].map((language) => ({
  id: language === "zh" ? "krdict-ko-zh" : "krdict-ko-ja",
  title: `KRDICT ${language}`,
  source_language: "ko",
  target_language: language,
  bytes: 15_000_000,
  sha256: "a".repeat(64),
  source_url: "https://github.com/Lyroxide/yomitan-ko-dic/releases/tag/1.0.0",
  license_url: "https://creativecommons.org/licenses/by-sa/2.0/kr/",
  installed: false,
}));
const source: DictionarySource = {
  id: 1, title: "KRDICT zh", revision: "1.0.0", source_language: "ko", target_language: "zh",
  entry_count: 100, imported_at: "2026-10-19T00:00:00Z",
};

function sectionProps(): SectionProps {
  return {
    locale: "zh-CN", selectionLookupEnabled: true, dictionaries: [], busy: false,
    message: "", progress: null, fileInputRef: { current: null },
    onSelectionLookupChange() {}, async onChoose() {}, async onRemove() {},
    quickInstall: { catalog, loading: false, error: false, installation: null, async onInstall() {}, async onReload() {} },
  };
}

function buttonTag(markup: string, label: string): string {
  const tag = markup.match(new RegExp(`<button[^>]*aria-label="${label}"[^>]*>`))?.[0];
  assert.ok(tag, `Missing button: ${label}`);
  return tag;
}

function browserTimers(t: TestContext) {
  const descriptor = Object.getOwnPropertyDescriptor(globalThis, "window");
  const timers = new Set<ReturnType<typeof setTimeout>>();
  Object.defineProperty(globalThis, "window", {
    configurable: true,
    value: {
      setTimeout(callback: () => void, milliseconds: number) {
        const timer = setTimeout(() => { timers.delete(timer); callback(); }, milliseconds === 250 ? 1 : milliseconds);
        timers.add(timer);
        return timer;
      },
      clearTimeout(timer: ReturnType<typeof setTimeout>) { timers.delete(timer); clearTimeout(timer); },
    },
  });
  t.after(() => {
    for (const timer of timers) clearTimeout(timer);
    if (descriptor) Object.defineProperty(globalThis, "window", descriptor);
    else Reflect.deleteProperty(globalThis, "window");
  });
  return timers;
}

test("dictionary settings offer two installs, preserve file import, and keep source details collapsed", async () => {
  const markup = await renderDictionary(sectionProps());
  assert.doesNotMatch(buttonTag(markup, "一键安装韩中词典"), /disabled/);
  assert.doesNotMatch(buttonTag(markup, "一键安装韩日词典"), /disabled/);
  assert.match(markup, /从文件导入 ZIP/);
  assert.match(markup, /自动下载并导入/);
  assert.match(markup, /韩国国立国语院/);
  assert.match(markup, /Lyroxide/);
  assert.match(markup, /href="https:\/\/creativecommons.org\/licenses\/by-sa\/2.0\/kr\/"/);
  assert.match(markup, /<details class="dictionary-source-details">/);
});

test("installation locks both buttons and file import while exposing the current stage and progress", async () => {
  const props = sectionProps();
  props.busy = true;
  props.quickInstall.installation = { catalog_id: "krdict-ko-zh", phase: "verifying", progress: 0.48 };
  const markup = await renderDictionary(props);
  assert.match(buttonTag(markup, "一键安装韩中词典"), /disabled/);
  assert.match(buttonTag(markup, "一键安装韩日词典"), /disabled/);
  assert.match(markup, /<button[^>]*disabled=""[^>]*>[\s\S]*?从文件导入 ZIP<\/button>/);
  assert.match(markup, /正在校验下载文件/);
  assert.match(markup, /role="progressbar"[^>]*aria-valuenow="48"/);
});

test("failure enables retry without changing another installed dictionary", async () => {
  const props = sectionProps();
  props.quickInstall.catalog = catalog.map((entry) => ({ ...entry, installed: entry.id === "krdict-ko-ja" }));
  props.quickInstall.installation = { catalog_id: "krdict-ko-zh", phase: "failed", progress: 0.45 };
  props.message = "词典下载失败，请检查网络后重试。";
  props.messageError = true;
  const markup = await renderDictionary(props);
  assert.doesNotMatch(buttonTag(markup, "重试安装韩中词典"), /disabled/);
  assert.match(buttonTag(markup, "韩日词典已安装"), /disabled/);
  assert.match(markup, /role="alert">词典下载失败/);
});

test("a removed dictionary can be installed again despite an old completed progress record", async () => {
  const props = sectionProps();
  props.quickInstall.installation = { catalog_id: "krdict-ko-zh", phase: "complete", progress: 1 };
  const markup = await renderDictionary(props);
  assert.doesNotMatch(buttonTag(markup, "一键安装韩中词典"), /disabled/);
});

test("install API sends a catalog ID and follows server download, verification, and import progress", async (t) => {
  const timers = browserTimers(t);
  const phases: (DictionaryImportProgress["phase"])[] = [];
  let finishPost!: (response: Response) => void;
  const postResponse = new Promise<Response>((resolve) => { finishPost = resolve; });
  let importId = "";
  const polledIds: string[] = [];
  let polls = 0;
  const statuses = [
    { progress: 0.2, phase: "downloading" },
    { progress: 0.48, phase: "verifying" },
    { progress: 0.8, phase: "importing" },
  ];
  t.mock.method(globalThis, "fetch", async (input: string | URL | Request, init?: RequestInit) => {
    const path = new URL(String(input)).pathname;
    if (path === "/api/dictionaries/install") {
      assert.equal(init?.method, "POST");
      assert.equal(init?.signal, undefined, "the long download must not use the normal 15-second timeout");
      assert.deepEqual(JSON.parse(String(init?.body)), { catalog_id: "krdict-ko-zh" });
      importId = new Headers(init?.headers).get("X-VRCS-Import-Id") ?? "";
      assert.ok(importId);
      return postResponse;
    }
    assert.ok(path.startsWith("/api/dictionaries/import/"));
    polledIds.push(decodeURIComponent(path.split("/").at(-1) ?? ""));
    if (polls++ === 0) return Response.json({ code: "dictionary.import_progress_not_found" }, { status: 404 });
    return Response.json(statuses[Math.min(polls - 2, statuses.length - 1)]);
  });
  const imported = await dictionaryApi.installDictionary("krdict-ko-zh", (status) => {
    phases.push(status.phase);
    if (status.phase === "importing") finishPost(Response.json(source));
  });
  assert.deepEqual(imported, source);
  assert.ok(polledIds.length > 1);
  assert.ok(polledIds.every((id) => id === importId));
  assert.deepEqual([...new Set(phases)], ["downloading", "verifying", "importing", "complete"]);
  assert.equal(timers.size, 0, "progress polling and request timers stop after success");
});

test("install failure preserves the structured error and a retry uses a new installation ID", async (t) => {
  const timers = browserTimers(t);
  const ids: string[] = [];
  t.mock.method(globalThis, "fetch", async (input: string | URL | Request, init?: RequestInit) => {
    if (new URL(String(input)).pathname === "/api/dictionaries/install") {
      ids.push(new Headers(init?.headers).get("X-VRCS-Import-Id") ?? "");
      return ids.length === 1
        ? Response.json({ code: "dictionary.download_integrity_failed", detail: "Checksum mismatch" }, { status: 502 })
        : Response.json(source);
    }
    return Response.json({ progress: 0.1, phase: "downloading" });
  });
  await assert.rejects(dictionaryApi.installDictionary("krdict-ko-zh"), { code: "dictionary.download_integrity_failed", status: 502 });
  assert.deepEqual(await dictionaryApi.installDictionary("krdict-ko-zh"), source);
  assert.equal(ids.length, 2);
  assert.ok(ids.every(Boolean));
  assert.notEqual(ids[0], ids[1]);
  assert.equal(timers.size, 0, "failed and successful requests both release polling timers");
});
