import assert from "node:assert/strict";
import test from "node:test";
import { ankiDictionaryContent, definitionGlosses, groupDictionaryEntries, hasAmbiguousKoreanHeadwords } from "../src/dictionary/dictionary.ts";

test("groups repeated headword entries and removes duplicate glosses", () => {
  const grouped = groupDictionaryEntries([
    { term: "便利", reading: "べんり", language: "ja", dictionary: "测试词典", definition: "方便\n有用" },
    { term: "便利", reading: "べんり", language: "ja", dictionary: "测试词典", definition: "有用；省事" },
  ]);

  assert.equal(grouped.length, 1);
  assert.deepEqual(definitionGlosses(grouped[0].definition), ["方便", "有用", "省事"]);
});

test("keeps different headwords and dictionary sources separate", () => {
  const grouped = groupDictionaryEntries([
    { term: "あ", reading: "", language: "ja", dictionary: "词典 A", definition: "感叹词" },
    { term: "亜", reading: "あ", language: "ja", dictionary: "词典 A", definition: "亚、次等" },
    { term: "あ", reading: "", language: "ja", dictionary: "词典 B", definition: "感叹词" },
  ]);

  assert.equal(grouped.length, 3);
});

test("collapses spelling variants with the same reading and definition", () => {
  const grouped = groupDictionaryEntries([
    { term: "話し相手", reading: "はなしあいて", language: "ja", dictionary: "测试词典", definition: "交谈的对象" },
    { term: "話しあいて", reading: "はなしあいて", language: "ja", dictionary: "测试词典", definition: "交谈的对象" },
  ]);

  assert.deepEqual(grouped.map((entry) => entry.term), ["話し相手"]);
});

test("keeps distinct Korean headwords even when their brief glosses match", () => {
  const grouped = groupDictionaryEntries([
    { term: "있다", language: "ko", dictionary: "Korean dictionary", definition: "to be" },
    { term: "이다", language: "ko", dictionary: "Korean dictionary", definition: "to be" },
    { term: "있다", language: "ko", dictionary: "Korean dictionary", definition: "to be" },
  ]);

  assert.deepEqual(grouped.map((entry) => entry.term), ["있다", "이다"]);
});

test("marks ambiguous Korean dictionary forms without treating duplicate sources as ambiguity", () => {
  const candidates = [
    { term: "들다", language: "ko", dictionary: "韩中词典", definition: "进入；拿起" },
    { term: "듣다", language: "ko", dictionary: "韩中词典", definition: "听" },
  ];
  assert.equal(hasAmbiguousKoreanHeadwords("들어요", candidates), true);
  assert.equal(hasAmbiguousKoreanHeadwords("듣다", candidates), false);
  assert.equal(hasAmbiguousKoreanHeadwords("들어요", [candidates[0]]), false);
  assert.equal(hasAmbiguousKoreanHeadwords("들어요", [candidates[0], { ...candidates[0], dictionary: "韩日词典" }]), false);
  assert.equal(hasAmbiguousKoreanHeadwords("들어요", []), false);
  assert.equal(hasAmbiguousKoreanHeadwords("聞きます", candidates), false);
});

test("labels every Korean dictionary form beside its own glosses on Anki cards", () => {
  const content = ankiDictionaryContent([
    { term: "들다", reading: "들다", language: "ko", dictionary: "韩中词典", definition: "进入；拿起" },
    { term: "듣다", reading: "듣따", language: "ko", dictionary: "韩中词典", definition: "听" },
  ], "内置词典", "들어요");

  assert.equal(content.definition, "【韩中词典 · 들다】\n1. 进入\n2. 拿起\n\n【韩中词典 · 듣다 (듣따)】\n1. 听");
  assert.equal(content.dictionary, "韩中词典");
});

test("keeps a single recovered headword explicit when the selected text is conjugated", () => {
  const entries = [{ term: "먹다", language: "ko", dictionary: "韩中词典", definition: "吃" }];
  assert.equal(ankiDictionaryContent(entries, "内置词典", "먹었어요").definition, "【韩中词典 · 먹다】\n1. 吃");
  assert.equal(ankiDictionaryContent(entries, "内置词典", "먹다").definition, "【韩中词典】\n1. 吃");
});
