import fs from "node:fs/promises";
import path from "node:path";

const FALLBACK_LOCALE = "zh-CN";
const SOURCE_DIR = path.resolve("i18n/source");

function isValidPlaceholderName(name) {
  return /^[A-Za-z0-9._-]+$/.test(name);
}

function normalizePlaceholderName(raw) {
  const candidate = raw.split(",")[0]?.trim();
  if (!candidate || !isValidPlaceholderName(candidate)) {
    return null;
  }
  return candidate;
}

function normalizePlaceholderSet(value) {
  const result = new Set();
  const legacyRegex = /\{\{\s*([^{}]+?)\s*\}\}/g;
  for (const match of value.matchAll(legacyRegex)) {
    const key = normalizePlaceholderName(match[1] ?? "");
    if (key) {
      result.add(key);
    }
  }

  const valueWithoutLegacy = value.replaceAll(legacyRegex, " ");
  const icuRegex = /\{\s*([^{}]+?)\s*\}/g;
  for (const match of valueWithoutLegacy.matchAll(icuRegex)) {
    const key = normalizePlaceholderName(match[1] ?? "");
    if (key) {
      result.add(key);
    }
  }
  return result;
}

function asSortedArray(set) {
  return [...set].sort((left, right) => left.localeCompare(right));
}

function setDiff(source, target) {
  return asSortedArray(new Set([...source].filter((item) => !target.has(item))));
}

function isValidKey(key) {
  return /^[A-Za-z0-9._-]+$/.test(key);
}

async function readLocaleNamespaces(localeDir) {
  const entries = await fs.readdir(localeDir, { withFileTypes: true });
  const namespaces = entries
    .filter((entry) => entry.isFile() && entry.name.endsWith(".json"))
    .map((entry) => entry.name.replace(/\.json$/u, ""))
    .sort((left, right) => left.localeCompare(right));
  return namespaces;
}

async function readNamespaceMap(locale, namespace) {
  const filePath = path.join(SOURCE_DIR, locale, `${namespace}.json`);
  const raw = await fs.readFile(filePath, "utf8");
  let parsed;
  try {
    parsed = JSON.parse(raw);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`${filePath} JSON 解析失败: ${message}`, {
      cause: error,
    });
  }

  if (parsed === null || Array.isArray(parsed) || typeof parsed !== "object") {
    throw new Error(`${filePath} 必须是 JSON 对象`);
  }

  const entries = Object.entries(parsed);
  const output = new Map();
  for (const [key, value] of entries) {
    if (!isValidKey(key)) {
      throw new Error(`${filePath} 存在非法 key: ${key}`);
    }
    if (typeof value !== "string") {
      throw new Error(`${filePath} 的 key=${key} 值必须为字符串`);
    }
    output.set(key, value);
  }

  return output;
}

async function main() {
  const localeEntries = await fs.readdir(SOURCE_DIR, { withFileTypes: true });
  const locales = localeEntries
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .sort((left, right) => left.localeCompare(right));

  const errors = [];
  if (locales.length === 0) {
    errors.push("i18n/source 下未找到任何 locale 目录");
  }
  if (!locales.includes(FALLBACK_LOCALE)) {
    errors.push(`缺少 fallback locale: ${FALLBACK_LOCALE}`);
  }
  if (errors.length > 0) {
    for (const error of errors) {
      console.error(`[i18n:check] ${error}`);
    }
    process.exit(1);
  }

  const localeNamespaces = new Map();
  for (const locale of locales) {
    const namespaceList = await readLocaleNamespaces(path.join(SOURCE_DIR, locale));
    localeNamespaces.set(locale, new Set(namespaceList));
  }

  const fallbackNamespaces = localeNamespaces.get(FALLBACK_LOCALE);
  for (const locale of locales) {
    const namespaces = localeNamespaces.get(locale);
    const missing = setDiff(fallbackNamespaces, namespaces);
    const extra = setDiff(namespaces, fallbackNamespaces);
    if (missing.length > 0) {
      errors.push(`${locale} 缺少 namespace: ${missing.join(", ")}`);
    }
    if (extra.length > 0) {
      errors.push(`${locale} 多出 namespace: ${extra.join(", ")}`);
    }
  }

  const fallbackNamespaceMaps = new Map();
  for (const namespace of fallbackNamespaces) {
    fallbackNamespaceMaps.set(namespace, await readNamespaceMap(FALLBACK_LOCALE, namespace));
  }

  for (const locale of locales) {
    for (const namespace of fallbackNamespaces) {
      const fallbackMap = fallbackNamespaceMaps.get(namespace);
      const localeMap = await readNamespaceMap(locale, namespace);
      const fallbackKeys = new Set(fallbackMap.keys());
      const localeKeys = new Set(localeMap.keys());

      const missing = setDiff(fallbackKeys, localeKeys);
      const extra = setDiff(localeKeys, fallbackKeys);
      if (missing.length > 0) {
        errors.push(`${locale}/${namespace} 缺少 key: ${missing.join(", ")}`);
      }
      if (extra.length > 0) {
        errors.push(`${locale}/${namespace} 多出 key: ${extra.join(", ")}`);
      }

      for (const key of fallbackKeys) {
        const fallbackPlaceholders = normalizePlaceholderSet(fallbackMap.get(key));
        const localePlaceholders = normalizePlaceholderSet(localeMap.get(key) ?? "");
        const fallbackToken = asSortedArray(fallbackPlaceholders).join(",");
        const localeToken = asSortedArray(localePlaceholders).join(",");
        if (fallbackToken !== localeToken) {
          errors.push(
            `${locale}/${namespace}:${key} 占位符不一致 (fallback=[${fallbackToken}] locale=[${localeToken}])`,
          );
        }
      }
    }
  }

  if (errors.length > 0) {
    for (const error of errors) {
      console.error(`[i18n:check] ${error}`);
    }
    process.exit(1);
  }

  console.log(
    `[i18n:check] OK - locales=${locales.join(", ")} namespaces=${asSortedArray(fallbackNamespaces).join(", ")}`,
  );
}

await main();
