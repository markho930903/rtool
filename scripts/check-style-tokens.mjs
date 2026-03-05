import { readdirSync, readFileSync, statSync } from "node:fs";
import { extname, join, relative } from "node:path";

const projectRoot = process.cwd();
const sourceRoot = join(projectRoot, "src");

const supportedExtensions = new Set([".ts", ".tsx", ".css"]);
const rawColorAllowedFiles = new Set(["src/styles/theme.css"]);
const allowedUiTextClasses = new Set(["ui-text-micro", "ui-text-caption", "ui-text-body-sm", "ui-text-body"]);
const uiTextClassRegex = /\bui-text-[a-z0-9-]+\b/g;

const rules = [
  {
    id: "palette-class",
    message: "禁止使用非主题语义色类名（示例：text-red-500、text-white）",
    regex:
      /\b(?:text|bg|border|ring|from|via|to)-(?:red|rose|pink|purple|violet|indigo|blue|cyan|teal|emerald|green|lime|yellow|amber|orange|gray|zinc|neutral|stone|slate|white|black)(?:-\d{1,3})?(?:\/\d{1,3})?\b/g,
  },
  {
    id: "raw-color-function",
    message: "禁止直接使用颜色函数（rgb/rgba/hsl/hsla）",
    regex: /\b(?:rgb|rgba|hsl|hsla)\s*\(/g,
    allowIn: rawColorAllowedFiles,
  },
  {
    id: "raw-shadow-arbitrary",
    message: "禁止在 shadow-[...] 中直接写颜色函数",
    regex: /\bshadow-\[[^\]\n]*(?:rgb|rgba|hsl|hsla)\s*\([^\]\n]*\]/g,
  },
  {
    id: "direct-token-arbitrary",
    message: "禁止在业务层使用 [var(--color|shadow|radius-*)]，请改用语义类名",
    regex: /\b(?:bg|text|border|ring|shadow|rounded)-\[(?:var\(--(?:color|shadow|radius)-[a-z0-9-]+\))\]/g,
  },
];

function walkFiles(rootDir) {
  const output = [];
  const entries = readdirSync(rootDir);

  for (const entry of entries) {
    const absolutePath = join(rootDir, entry);
    const fileStat = statSync(absolutePath);
    if (fileStat.isDirectory()) {
      output.push(...walkFiles(absolutePath));
      continue;
    }

    if (!supportedExtensions.has(extname(entry))) {
      continue;
    }

    output.push(absolutePath);
  }

  return output;
}

function positionOf(content, index) {
  const untilMatch = content.slice(0, index);
  const lines = untilMatch.split("\n");
  const line = lines.length;
  const column = lines[lines.length - 1].length + 1;
  return { line, column };
}

const candidates = walkFiles(sourceRoot);
const violations = [];

for (const absolutePath of candidates) {
  const relativePath = relative(projectRoot, absolutePath).replaceAll("\\", "/");
  const content = readFileSync(absolutePath, "utf8");

  for (const rule of rules) {
    if (rule.allowIn?.has(relativePath)) {
      continue;
    }

    for (const match of content.matchAll(rule.regex)) {
      if (match.index === undefined) {
        continue;
      }

      const { line, column } = positionOf(content, match.index);
      violations.push({
        file: relativePath,
        line,
        column,
        ruleId: rule.id,
        message: rule.message,
        sample: match[0],
      });
    }
  }

  for (const match of content.matchAll(uiTextClassRegex)) {
    if (match.index === undefined) {
      continue;
    }

    const className = match[0];
    if (allowedUiTextClasses.has(className)) {
      continue;
    }

    const { line, column } = positionOf(content, match.index);
    violations.push({
      file: relativePath,
      line,
      column,
      ruleId: "unknown-ui-text-class",
      message: `未定义的 ui-text 语义类：${className}，仅允许 ${Array.from(allowedUiTextClasses).join(", ")}`,
      sample: className,
    });
  }
}

if (violations.length > 0) {
  console.error("✖ token style check failed:");
  for (const item of violations) {
    console.error(
      `- ${item.file}:${item.line}:${item.column} [${item.ruleId}] ${item.message}\n  sample: ${item.sample}`,
    );
  }
  process.exit(1);
}

console.log("✓ token style check passed");
