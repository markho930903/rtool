import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const CONTRACTS_FILE = "src/contracts/index.ts";
const MODELS_SOURCE_FILE = "src-tauri/crates/rtool-contracts/src/models.rs";
const ERRORS_SOURCE_FILE = "src-tauri/crates/rtool-contracts/src/errors.rs";
const SUPPLEMENTAL_STRUCT_SOURCES = [
  {
    file: "src-tauri/crates/rtool-kernel/src/i18n.rs",
    structNames: ["LocaleStateDto"],
  },
];
const TYPE_ALIAS_OVERRIDES = new Map([
  ["AppLocalePreference", "string"],
  ["ResolvedAppLocale", "string"],
]);
const CHECK_MODE = process.argv.includes("--check");

function readText(relativePath) {
  return fs.readFileSync(path.join(root, relativePath), "utf8");
}

function writeText(relativePath, content) {
  fs.writeFileSync(path.join(root, relativePath), content, "utf8");
}

function fail(message) {
  throw new Error(`[contracts:generate] ${message}`);
}

function parseKernelFeatureKeys() {
  const source = readText("src-tauri/crates/rtool-kernel/src/feature.rs");
  const constMatch = source.match(/pub const FEATURE_KEYS:[\s\S]*?=\s*\[([\s\S]*?)\];/m);
  let keys = constMatch
    ? Array.from(constMatch[1].matchAll(/"([^"]+)"/g)).map((item) => item[1])
    : [];

  if (keys.length === 0) {
    const macroMatch = source.match(/define_feature_keys!\(([\s\S]*?)\);/m);
    if (macroMatch) {
      keys = Array.from(macroMatch[1].matchAll(/=>\s*"([^"]+)"/g)).map((item) => item[1]);
    }
  }

  if (keys.length === 0) {
    fail("FEATURE_KEYS is empty or unparsable");
  }

  return keys;
}

function toSnakeCase(value) {
  return value
    .replace(/([a-z0-9])([A-Z])/g, "$1_$2")
    .replace(/([A-Z])([A-Z][a-z])/g, "$1_$2")
    .toLowerCase();
}

function toCamelCase(value) {
  return value.replace(/_([a-z])/g, (_, char) => char.toUpperCase());
}

function toLowerCamelCase(value) {
  if (value.length === 0) {
    return value;
  }
  return value[0].toLowerCase() + value.slice(1);
}

function applyRenameRule(name, rule) {
  if (!rule) {
    return name;
  }
  if (rule === "snake_case") {
    return toSnakeCase(name);
  }
  if (rule === "camelCase") {
    if (name.includes("_")) {
      return toCamelCase(name);
    }
    return toLowerCamelCase(name);
  }
  return name;
}

function stripModulePath(typeName) {
  const normalized = typeName.trim();
  if (!normalized.includes("::")) {
    return normalized;
  }
  const segments = normalized.split("::");
  return segments[segments.length - 1];
}

function splitTopLevelGenericArgs(source) {
  const args = [];
  let depth = 0;
  let chunk = "";

  for (const char of source) {
    if (char === "<") {
      depth += 1;
      chunk += char;
      continue;
    }
    if (char === ">") {
      depth -= 1;
      chunk += char;
      continue;
    }
    if (char === "," && depth === 0) {
      args.push(chunk.trim());
      chunk = "";
      continue;
    }
    chunk += char;
  }

  if (chunk.trim().length > 0) {
    args.push(chunk.trim());
  }

  return args;
}

function splitTopLevelCommaList(source) {
  const parts = [];
  let depthBrace = 0;
  let depthParen = 0;
  let depthBracket = 0;
  let current = "";

  for (const char of source) {
    if (char === "{") {
      depthBrace += 1;
    } else if (char === "}") {
      depthBrace -= 1;
    } else if (char === "(") {
      depthParen += 1;
    } else if (char === ")") {
      depthParen -= 1;
    } else if (char === "[") {
      depthBracket += 1;
    } else if (char === "]") {
      depthBracket -= 1;
    }

    if (char === "," && depthBrace === 0 && depthParen === 0 && depthBracket === 0) {
      if (current.trim().length > 0) {
        parts.push(current.trim());
      }
      current = "";
      continue;
    }

    current += char;
  }

  if (current.trim().length > 0) {
    parts.push(current.trim());
  }

  return parts;
}

function unwrapGeneric(typeName, genericName) {
  const normalized = typeName.trim();
  const prefix = `${genericName}<`;
  if (!normalized.startsWith(prefix) || !normalized.endsWith(">")) {
    return null;
  }
  return normalized.slice(prefix.length, -1);
}

function mapRustTypeToTs(typeName, optionMode) {
  const normalized = stripModulePath(typeName.trim());

  const optionInner = unwrapGeneric(normalized, "Option");
  if (optionInner !== null) {
    const mappedInner = mapRustTypeToTs(optionInner, optionMode).tsType;
    if (optionMode === "nullable") {
      return {
        optional: false,
        tsType: `${mappedInner} | null`,
      };
    }
    return {
      optional: true,
      tsType: mappedInner,
    };
  }

  const vecInner = unwrapGeneric(normalized, "Vec");
  if (vecInner !== null) {
    return {
      optional: false,
      tsType: `Array<${mapRustTypeToTs(vecInner, optionMode).tsType}>`,
    };
  }

  const mapInner = unwrapGeneric(normalized, "HashMap");
  if (mapInner !== null) {
    const [, valueType = "unknown"] = splitTopLevelGenericArgs(mapInner);
    return {
      optional: false,
      tsType: `Record<string, ${mapRustTypeToTs(valueType, optionMode).tsType}>`,
    };
  }

  if (normalized === "String" || normalized === "str" || normalized === "&str") {
    return { optional: false, tsType: "string" };
  }

  if (normalized === "bool") {
    return { optional: false, tsType: "boolean" };
  }

  if (
    ["u8", "u16", "u32", "u64", "usize", "i8", "i16", "i32", "i64", "isize", "f32", "f64"].includes(
      normalized,
    )
  ) {
    return { optional: false, tsType: "number" };
  }

  if (normalized === "Value" || normalized === "serde_json::Value") {
    return { optional: false, tsType: "JsonValue" };
  }

  if (TYPE_ALIAS_OVERRIDES.has(normalized)) {
    return { optional: false, tsType: TYPE_ALIAS_OVERRIDES.get(normalized) };
  }

  return { optional: false, tsType: normalized };
}

function parseSerdeRename(attrs) {
  const match = attrs.match(/rename\s*=\s*"([^"]+)"/);
  return match ? match[1] : null;
}

function parseSerdeRenameAll(attrs) {
  const match = attrs.match(/rename_all\s*=\s*"([^"]+)"/);
  return match ? match[1] : null;
}

function parseSerdeTag(attrs) {
  const match = attrs.match(/tag\s*=\s*"([^"]+)"/);
  return match ? match[1] : null;
}

function parseNamedFields(body, renameRule, optionMode) {
  const fields = [];
  const fieldRegex =
    /((?:\s*#\[[^\n]+\]\s*\n)*)\s*(?:pub(?:\([^)]+\))?\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*:\s*([^,]+),/g;

  for (const fieldMatch of body.matchAll(fieldRegex)) {
    const fieldAttrs = fieldMatch[1] ?? "";
    const rustFieldName = fieldMatch[2];
    const rustFieldType = fieldMatch[3].trim();
    const mapped = mapRustTypeToTs(rustFieldType, optionMode);
    const rename = parseSerdeRename(fieldAttrs);
    const tsName = rename ?? applyRenameRule(rustFieldName, renameRule);

    fields.push({
      rustName: rustFieldName,
      tsName,
      tsType: mapped.tsType,
      optional: mapped.optional,
    });
  }

  return fields;
}

function parseStructDefs(source, optionMode) {
  const structs = [];
  const structRegex =
    /((?:\s*#\[[^\n]+\]\s*\n)*)\s*pub(?:\([^)]+\))?\s+struct\s+(\w+)\s*\{([\s\S]*?)\n\}/g;

  for (const match of source.matchAll(structRegex)) {
    const attrs = match[1] ?? "";
    const name = match[2];
    const body = match[3] ?? "";
    const renameRule = parseSerdeRenameAll(attrs);

    structs.push({
      kind: "struct",
      index: match.index,
      name,
      fields: parseNamedFields(body, renameRule, optionMode),
    });
  }

  return structs;
}

function parseEnumVariants(body, renameRule, optionMode) {
  const variants = [];

  for (const rawChunk of splitTopLevelCommaList(body)) {
    let chunk = rawChunk.trimStart();
    while (chunk.startsWith("#[")) {
      const next = chunk.replace(/^#\[[^\n]+\]\s*/, "");
      if (next === chunk) {
        break;
      }
      chunk = next.trimStart();
    }
    chunk = chunk.trim();
    if (chunk.length === 0) {
      continue;
    }

    const variantMatch = chunk.match(/^([A-Za-z_][A-Za-z0-9_]*)\s*([\s\S]*)$/);
    if (!variantMatch) {
      continue;
    }

    const rustName = variantMatch[1];
    const tsName = applyRenameRule(rustName, renameRule);
    const rest = variantMatch[2].trim();

    if (rest.length === 0) {
      variants.push({ kind: "unit", rustName, tsName });
      continue;
    }

    if (rest.startsWith("(")) {
      const tupleBody = rest.slice(1, -1).trim();
      variants.push({
        kind: "tuple",
        rustName,
        tsName,
        tupleType: mapRustTypeToTs(tupleBody, optionMode).tsType,
      });
      continue;
    }

    if (rest.startsWith("{")) {
      const namedBody = rest.slice(1, -1);
      variants.push({
        kind: "struct",
        rustName,
        tsName,
        fields: parseNamedFields(namedBody, null, optionMode),
      });
      continue;
    }

    fail(`Unsupported enum variant shape: ${chunk}`);
  }

  return variants;
}

function parseEnumDefs(source, optionMode) {
  const enums = [];
  const enumRegex =
    /((?:\s*#\[[^\n]+\]\s*\n)*)\s*pub(?:\([^)]+\))?\s+enum\s+(\w+)\s*\{([\s\S]*?)\n\}/g;

  for (const match of source.matchAll(enumRegex)) {
    const attrs = match[1] ?? "";
    const name = match[2];
    const body = match[3] ?? "";
    const renameRule = parseSerdeRenameAll(attrs);
    const tag = parseSerdeTag(attrs);

    enums.push({
      kind: "enum",
      index: match.index,
      name,
      tag,
      variants: parseEnumVariants(body, renameRule, optionMode),
    });
  }

  return enums;
}

function renderStructType(structDef) {
  const rows = structDef.fields.map((field) => {
    const optionalMark = field.optional ? "?" : "";
    return `  ${field.tsName}${optionalMark}: ${field.tsType};`;
  });

  return `export type ${structDef.name} = {\n${rows.join("\n")}\n};`;
}

function renderEnumType(enumDef) {
  if (enumDef.tag) {
    const variantRows = enumDef.variants.map((variant) => {
      if (variant.kind === "unit") {
        return `  | { ${enumDef.tag}: "${variant.tsName}" }`;
      }
      if (variant.kind === "tuple") {
        return `  | { ${enumDef.tag}: "${variant.tsName}"; value: ${variant.tupleType} }`;
      }
      const fieldRows = variant.fields.map((field) => {
        const optionalMark = field.optional ? "?" : "";
        return `${field.tsName}${optionalMark}: ${field.tsType}`;
      });
      const suffix = fieldRows.length > 0 ? `; ${fieldRows.join("; ")}` : "";
      return `  | { ${enumDef.tag}: "${variant.tsName}"${suffix} }`;
    });

    return `export type ${enumDef.name} =\n${variantRows.join("\n")};`;
  }

  const nonUnit = enumDef.variants.filter((variant) => variant.kind !== "unit");
  if (nonUnit.length > 0) {
    fail(`Unsupported non-tagged enum shape: ${enumDef.name}`);
  }

  const literals = enumDef.variants.map((variant) => `  | "${variant.tsName}"`);
  return `export type ${enumDef.name} =\n${literals.join("\n")};`;
}

function resolveFeatureRequestSourceFile(featureKey) {
  const moduleName = featureKey;
  const candidateFiles = [
    `src-tauri/crates/rtool-host-tauri/src/features/${moduleName}/types.rs`,
    `src-tauri/crates/rtool-host-tauri/src/features/${moduleName}/api.rs`,
  ];

  for (const file of candidateFiles) {
    if (fs.existsSync(path.join(root, file))) {
      return file;
    }
  }

  fail(`Missing request source file for feature ${featureKey} (${candidateFiles.join(", ")})`);
}

function parseRequestEnumName(source, featureKey) {
  const enumMatch = source.match(
    /#\s*\[\s*serde\s*\(\s*tag\s*=\s*"kind"\s*,\s*content\s*=\s*"payload"[\s\S]*?\)\s*\][\s\S]*?enum\s+(\w+Request)\s*\{/m,
  );
  if (!enumMatch) {
    fail(`Missing serde-tagged request enum for feature ${featureKey}`);
  }
  return enumMatch[1];
}

function parseEnumVariantsForRequests(source, enumName) {
  const enumRegex = new RegExp(
    `((?:\\s*#\\[[^\\n]+\\]\\s*\\n)*)\\s*pub(?:\\([^)]+\\))?\\s+enum\\s+${enumName}\\s*\\{([\\s\\S]*?)\\n\\}`,
  );
  const match = source.match(enumRegex);
  if (!match) {
    fail(`Missing enum ${enumName}`);
  }

  const attrs = match[1] ?? "";
  const body = match[2] ?? "";
  const renameRule = parseSerdeRenameAll(attrs);

  const variants = [];
  for (const rawChunk of splitTopLevelCommaList(body)) {
    const chunk = rawChunk.trim();
    if (chunk.length === 0) {
      continue;
    }

    const variantMatch = chunk.match(/^([A-Za-z_][A-Za-z0-9_]*)(?:\(([^)]+)\))?$/);
    if (!variantMatch) {
      fail(`Unsupported request variant shape in ${enumName}: ${chunk}`);
    }

    const rawName = variantMatch[1];
    const payloadType = variantMatch[2]?.trim() ?? null;
    variants.push({
      kind: applyRenameRule(rawName, renameRule),
      payloadType: payloadType ? stripModulePath(payloadType) : null,
    });
  }

  return variants;
}

function renderPayloadType(payloadStruct) {
  if (!payloadStruct || payloadStruct.fields.length === 0) {
    return "{}";
  }

  const items = payloadStruct.fields.map((field) => {
    const optionalMark = field.optional ? "?" : "";
    return `${field.tsName}${optionalMark}: ${field.tsType}`;
  });
  return `{ ${items.join("; ")} }`;
}

function renderRequestType(enumName, variants, structs) {
  const requestTypeName = `${enumName}Dto`;
  const variantRows = variants.map((variant) => {
    if (!variant.payloadType) {
      return `CommandNoPayload<"${variant.kind}">`;
    }

    const payloadStruct = structs.find((item) => item.name === variant.payloadType);
    if (!payloadStruct) {
      fail(`Missing payload struct ${variant.payloadType} for ${enumName}.${variant.kind}`);
    }

    return `CommandWithPayload<"${variant.kind}", ${renderPayloadType(payloadStruct)}>`;
  });

  return {
    requestTypeName,
    requestTypeDef: `export type ${requestTypeName} =\n${variantRows.map((value) => `  | ${value}`).join("\n")};`,
  };
}

function parseRequestSourceForFeature(featureKey) {
  const file = resolveFeatureRequestSourceFile(featureKey);
  const source = readText(file);
  const enumName = parseRequestEnumName(source, featureKey);
  const structs = parseStructDefs(source, "optional");
  const variants = parseEnumVariantsForRequests(source, enumName);
  const renderedRequestType = renderRequestType(enumName, variants, structs);

  return {
    feature: featureKey,
    enumName,
    variants,
    requestTypeName: renderedRequestType.requestTypeName,
    requestType: renderedRequestType.requestTypeDef,
  };
}

function parseRequestContracts(keys) {
  return keys.map((featureKey) => parseRequestSourceForFeature(featureKey));
}

function validateFeatureCoverage(keys, requests) {
  const requestFeatures = requests.map((item) => item.feature).sort();
  const missing = keys.filter((key) => !requestFeatures.includes(key)).sort();
  const extra = requestFeatures.filter((feature) => !keys.includes(feature)).sort();

  if (missing.length > 0) {
    fail(`Missing request contracts for features: ${missing.join(", ")}`);
  }

  if (extra.length > 0) {
    fail(`Request contracts include unknown features: ${extra.join(", ")}`);
  }
}

function parseModelContracts() {
  const modelSource = readText(MODELS_SOURCE_FILE);
  const errorSource = readText(ERRORS_SOURCE_FILE);

  const modelStructs = parseStructDefs(modelSource, "nullable");
  const modelEnums = parseEnumDefs(modelSource, "nullable");
  const allModelItems = [...modelStructs, ...modelEnums].sort((left, right) => left.index - right.index);

  const errorStructs = parseStructDefs(errorSource, "nullable");
  const rendered = [];
  for (const errorStruct of errorStructs) {
    rendered.push(renderStructType(errorStruct));
  }

  for (const item of allModelItems) {
    if (item.kind === "struct") {
      rendered.push(renderStructType(item));
    } else {
      rendered.push(renderEnumType(item));
    }
  }

  for (const supplementalSource of SUPPLEMENTAL_STRUCT_SOURCES) {
    const source = readText(supplementalSource.file);
    const structs = parseStructDefs(source, "nullable");
    for (const structName of supplementalSource.structNames) {
      const structDef = structs.find((item) => item.name === structName);
      if (!structDef) {
        fail(`Missing supplemental struct ${structName} in ${supplementalSource.file}`);
      }
      rendered.push(renderStructType(structDef));
    }
  }

  return rendered;
}

function renderGeneratedModelsBlock(renderedTypes) {
  const lines = [];
  lines.push("// <generated-models:start>");
  lines.push(
    "export type JsonValue = number | string | boolean | Array<JsonValue> | { [key in string]?: JsonValue } | null;",
  );
  lines.push("");
  for (const typeDef of renderedTypes) {
    lines.push(typeDef);
    lines.push("");
  }
  lines.push("// <generated-models:end>");
  lines.push("");
  return lines.join("\n");
}

function renderGeneratedCommandContracts(keys, requests) {
  const requestTypeNameByFeature = new Map(
    requests.map((item) => [item.feature, item.requestTypeName]),
  );

  const lines = [];
  lines.push("// <generated-contracts:start>");
  lines.push("export type AppFeatureKey =");
  for (let index = 0; index < keys.length; index += 1) {
    const key = keys[index];
    const end = index === keys.length - 1 ? ";" : "";
    lines.push(`  | "${key}"${end}`);
  }
  lines.push("");
  lines.push("type CommandNoPayload<K extends string> = { kind: K };");
  lines.push("type CommandWithPayload<K extends string, P> = { kind: K; payload: P };");
  lines.push("");
  lines.push("export type InvokeMetaDto = {");
  lines.push("  requestId?: string;");
  lines.push("  windowLabel?: string;");
  lines.push("};");
  lines.push("");
  lines.push("export type AppFeatureRequestMap = {");
  for (const key of keys) {
    const requestTypeName = requestTypeNameByFeature.get(key);
    if (!requestTypeName) {
      fail(`Missing request type mapping for feature: ${key}`);
    }
    lines.push(`  "${key}": ${requestTypeName};`);
  }
  lines.push("};");
  lines.push("");
  for (const requestType of requests.map((item) => item.requestType)) {
    lines.push(requestType);
    lines.push("");
  }
  lines.push("// <generated-contracts:end>");
  lines.push("");

  return lines.join("\n");
}

function renderContractsFile(modelBlock, commandBlock) {
  return [
    "/* eslint-disable */",
    "// 前后端合同类型单一事实源。",
    "// 该文件由 `pnpm contracts:generate` 全量生成，请勿手工修改。",
    "",
    modelBlock.trimEnd(),
    "",
    commandBlock.trimEnd(),
    "",
  ].join("\n");
}

const keys = parseKernelFeatureKeys();
const requests = parseRequestContracts(keys);
validateFeatureCoverage(keys, requests);

const commandBlock = renderGeneratedCommandContracts(keys, requests);

const modelTypes = parseModelContracts();
const modelBlock = renderGeneratedModelsBlock(modelTypes);

const contractsSource = readText(CONTRACTS_FILE);
const nextContracts = renderContractsFile(modelBlock, commandBlock);

if (CHECK_MODE) {
  if (nextContracts !== contractsSource) {
    fail(`contracts out of date, run: pnpm contracts:generate (${keys.join(", ")})`);
  }
  console.log(
    `[contracts:generate] check OK - contracts are synchronized (${keys.join(", ")})`,
  );
  process.exit(0);
}

if (nextContracts !== contractsSource) {
  writeText(CONTRACTS_FILE, nextContracts);
}

console.log(
  `[contracts:generate] synced Rust contracts (${keys.join(", ")}) and regenerated models + request DTO unions`,
);
