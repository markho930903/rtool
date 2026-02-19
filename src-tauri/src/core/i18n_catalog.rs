use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard};

const OVERLAY_MAX_BYTES: usize = 512 * 1024;

struct BuiltinBundle {
    locale: &'static str,
    namespace: &'static str,
    content: &'static str,
}

const BUILTIN_BUNDLES: &[BuiltinBundle] = &[
    BuiltinBundle {
        locale: "zh-CN",
        namespace: "common",
        content: include_str!("../../../i18n/source/zh-CN/common.json"),
    },
    BuiltinBundle {
        locale: "zh-CN",
        namespace: "layout",
        content: include_str!("../../../i18n/source/zh-CN/layout.json"),
    },
    BuiltinBundle {
        locale: "zh-CN",
        namespace: "home",
        content: include_str!("../../../i18n/source/zh-CN/home.json"),
    },
    BuiltinBundle {
        locale: "zh-CN",
        namespace: "tools",
        content: include_str!("../../../i18n/source/zh-CN/tools.json"),
    },
    BuiltinBundle {
        locale: "zh-CN",
        namespace: "transfer",
        content: include_str!("../../../i18n/source/zh-CN/transfer.json"),
    },
    BuiltinBundle {
        locale: "zh-CN",
        namespace: "logs",
        content: include_str!("../../../i18n/source/zh-CN/logs.json"),
    },
    BuiltinBundle {
        locale: "zh-CN",
        namespace: "settings",
        content: include_str!("../../../i18n/source/zh-CN/settings.json"),
    },
    BuiltinBundle {
        locale: "zh-CN",
        namespace: "clipboard",
        content: include_str!("../../../i18n/source/zh-CN/clipboard.json"),
    },
    BuiltinBundle {
        locale: "zh-CN",
        namespace: "palette",
        content: include_str!("../../../i18n/source/zh-CN/palette.json"),
    },
    BuiltinBundle {
        locale: "zh-CN",
        namespace: "not_found",
        content: include_str!("../../../i18n/source/zh-CN/not_found.json"),
    },
    BuiltinBundle {
        locale: "zh-CN",
        namespace: "native",
        content: include_str!("../../../i18n/source/zh-CN/native.json"),
    },
    BuiltinBundle {
        locale: "en-US",
        namespace: "common",
        content: include_str!("../../../i18n/source/en-US/common.json"),
    },
    BuiltinBundle {
        locale: "en-US",
        namespace: "layout",
        content: include_str!("../../../i18n/source/en-US/layout.json"),
    },
    BuiltinBundle {
        locale: "en-US",
        namespace: "home",
        content: include_str!("../../../i18n/source/en-US/home.json"),
    },
    BuiltinBundle {
        locale: "en-US",
        namespace: "tools",
        content: include_str!("../../../i18n/source/en-US/tools.json"),
    },
    BuiltinBundle {
        locale: "en-US",
        namespace: "transfer",
        content: include_str!("../../../i18n/source/en-US/transfer.json"),
    },
    BuiltinBundle {
        locale: "en-US",
        namespace: "logs",
        content: include_str!("../../../i18n/source/en-US/logs.json"),
    },
    BuiltinBundle {
        locale: "en-US",
        namespace: "settings",
        content: include_str!("../../../i18n/source/en-US/settings.json"),
    },
    BuiltinBundle {
        locale: "en-US",
        namespace: "clipboard",
        content: include_str!("../../../i18n/source/en-US/clipboard.json"),
    },
    BuiltinBundle {
        locale: "en-US",
        namespace: "palette",
        content: include_str!("../../../i18n/source/en-US/palette.json"),
    },
    BuiltinBundle {
        locale: "en-US",
        namespace: "not_found",
        content: include_str!("../../../i18n/source/en-US/not_found.json"),
    },
    BuiltinBundle {
        locale: "en-US",
        namespace: "native",
        content: include_str!("../../../i18n/source/en-US/native.json"),
    },
];

#[derive(Debug, Clone, Default)]
struct CatalogLayer {
    values: HashMap<String, HashMap<String, String>>,
    namespaces: HashMap<String, BTreeSet<String>>,
}

impl CatalogLayer {
    fn insert_namespace(
        &mut self,
        locale: &str,
        namespace: &str,
        entries: HashMap<String, String>,
    ) {
        let locale_values = self.values.entry(locale.to_string()).or_default();
        for (key, value) in entries {
            locale_values.insert(key, value);
        }

        self.namespaces
            .entry(locale.to_string())
            .or_default()
            .insert(namespace.to_string());
    }

    fn get(&self, locale: &str, key: &str) -> Option<&str> {
        self.values
            .get(locale)
            .and_then(|bucket| bucket.get(key))
            .map(String::as_str)
    }

    fn namespaces_for(&self, locale: &str) -> Vec<String> {
        let mut values = self
            .namespaces
            .get(locale)
            .map(|set| set.iter().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        values.sort();
        values
    }

    fn locale_namespaces(&self) -> Vec<LocaleNamespaces> {
        let mut locales = self
            .namespaces
            .keys()
            .map(|locale| LocaleNamespaces {
                locale: locale.clone(),
                namespaces: self.namespaces_for(locale),
            })
            .collect::<Vec<_>>();
        locales.sort_by(|left, right| left.locale.cmp(&right.locale));
        locales
    }
}

#[derive(Debug, Clone)]
struct I18nCatalog {
    builtin: CatalogLayer,
    overlay: CatalogLayer,
    overlay_root: PathBuf,
}

impl I18nCatalog {
    fn lookup_in_locale(&self, locale: &str, key: &str) -> Option<&str> {
        self.overlay
            .get(locale, key)
            .or_else(|| self.builtin.get(locale, key))
    }

    fn lookup_with_fallback(&self, locale: &str, fallback_locale: &str, key: &str) -> Option<&str> {
        self.lookup_in_locale(locale, key)
            .or_else(|| self.lookup_in_locale(fallback_locale, key))
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocaleNamespaces {
    pub locale: String,
    pub namespaces: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocaleCatalogList {
    pub builtin_locales: Vec<LocaleNamespaces>,
    pub overlay_locales: Vec<LocaleNamespaces>,
    pub effective_locales: Vec<LocaleNamespaces>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportLocaleResult {
    pub success: bool,
    pub locale: String,
    pub namespace: String,
    pub imported_keys: u32,
    pub warnings: Vec<String>,
    pub effective_locale_namespaces: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReloadLocalesResult {
    pub success: bool,
    pub overlay_locales: Vec<LocaleNamespaces>,
    pub reloaded_files: u32,
    pub warnings: Vec<String>,
}

#[derive(Debug, Default)]
struct OverlayLoadResult {
    layer: CatalogLayer,
    loaded_files: u32,
    warnings: Vec<String>,
}

static CATALOG: OnceLock<RwLock<I18nCatalog>> = OnceLock::new();

pub fn initialize(app_data_dir: &Path) -> Result<()> {
    let builtin = load_builtin_layer()?;
    let overlay_root = app_data_dir.join("locales");
    fs::create_dir_all(&overlay_root)
        .with_context(|| format!("创建语言目录失败: {}", overlay_root.display()))?;
    let overlay = load_overlay_layer(&overlay_root)?;

    let catalog = I18nCatalog {
        builtin,
        overlay: overlay.layer,
        overlay_root,
    };

    if !overlay.warnings.is_empty() {
        for warning in overlay.warnings {
            tracing::warn!(event = "i18n_overlay_load_warning", detail = warning);
        }
    }

    match CATALOG.get() {
        Some(lock) => {
            let mut guard = write_guard(lock);
            *guard = catalog;
        }
        None => {
            CATALOG
                .set(RwLock::new(catalog))
                .map_err(|_| anyhow::anyhow!("初始化语言目录失败: catalog 已存在"))?;
        }
    }

    Ok(())
}

pub fn translate(locale: &str, fallback_locale: &str, key: &str) -> Option<String> {
    let lock = CATALOG.get()?;
    let guard = read_guard(lock);
    guard
        .lookup_with_fallback(locale, fallback_locale, key)
        .map(ToString::to_string)
}

pub fn list_locales() -> Result<LocaleCatalogList> {
    let lock = get_catalog_lock()?;
    let guard = read_guard(lock);
    Ok(build_catalog_list(&guard))
}

pub fn reload_overlays() -> Result<ReloadLocalesResult> {
    let lock = get_catalog_lock()?;
    let mut guard = write_guard(lock);
    let overlay = load_overlay_layer(&guard.overlay_root)?;
    guard.overlay = overlay.layer.clone();

    if !overlay.warnings.is_empty() {
        for warning in &overlay.warnings {
            tracing::warn!(event = "i18n_overlay_reload_warning", detail = warning);
        }
    }

    Ok(ReloadLocalesResult {
        success: true,
        overlay_locales: overlay.layer.locale_namespaces(),
        reloaded_files: overlay.loaded_files,
        warnings: overlay.warnings,
    })
}

pub fn import_locale_file(
    locale: &str,
    namespace: &str,
    content: &str,
    replace: bool,
    fallback_locale: &str,
) -> Result<ImportLocaleResult> {
    validate_locale_code(locale)?;
    validate_namespace(namespace)?;

    if content.len() > OVERLAY_MAX_BYTES {
        anyhow::bail!(
            "导入失败: 文件过大 ({} bytes), 上限 {} bytes",
            content.len(),
            OVERLAY_MAX_BYTES
        );
    }

    let entries = parse_translation_json(content, &format!("{}:{}", locale, namespace), true)?;
    if entries.is_empty() {
        anyhow::bail!("导入失败: 翻译文件不能为空");
    }

    let lock = get_catalog_lock()?;
    let mut guard = write_guard(lock);

    let warnings = collect_placeholder_warnings(&guard, fallback_locale, &entries);
    persist_overlay_namespace(&guard.overlay_root, locale, namespace, &entries, replace)?;

    let overlay = load_overlay_layer(&guard.overlay_root)?;
    if !overlay.warnings.is_empty() {
        for warning in &overlay.warnings {
            tracing::warn!(event = "i18n_overlay_reload_warning", detail = warning);
        }
    }
    guard.overlay = overlay.layer;

    let mut effective_namespaces = BTreeSet::new();
    for value in guard.builtin.namespaces_for(locale) {
        effective_namespaces.insert(value);
    }
    for value in guard.overlay.namespaces_for(locale) {
        effective_namespaces.insert(value);
    }

    Ok(ImportLocaleResult {
        success: true,
        locale: locale.to_string(),
        namespace: namespace.to_string(),
        imported_keys: entries.len() as u32,
        warnings,
        effective_locale_namespaces: effective_namespaces.into_iter().collect(),
    })
}

fn get_catalog_lock() -> Result<&'static RwLock<I18nCatalog>> {
    CATALOG.get().context("语言目录尚未初始化")
}

fn read_guard(lock: &RwLock<I18nCatalog>) -> RwLockReadGuard<'_, I18nCatalog> {
    match lock.read() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn write_guard(lock: &RwLock<I18nCatalog>) -> RwLockWriteGuard<'_, I18nCatalog> {
    match lock.write() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn build_catalog_list(catalog: &I18nCatalog) -> LocaleCatalogList {
    let builtin_locales = catalog.builtin.locale_namespaces();
    let overlay_locales = catalog.overlay.locale_namespaces();

    let mut effective_by_locale: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for item in &builtin_locales {
        let entry = effective_by_locale.entry(item.locale.clone()).or_default();
        for namespace in &item.namespaces {
            entry.insert(namespace.clone());
        }
    }
    for item in &overlay_locales {
        let entry = effective_by_locale.entry(item.locale.clone()).or_default();
        for namespace in &item.namespaces {
            entry.insert(namespace.clone());
        }
    }

    let effective_locales = effective_by_locale
        .into_iter()
        .map(|(locale, namespaces)| LocaleNamespaces {
            locale,
            namespaces: namespaces.into_iter().collect(),
        })
        .collect::<Vec<_>>();

    LocaleCatalogList {
        builtin_locales,
        overlay_locales,
        effective_locales,
    }
}

fn load_builtin_layer() -> Result<CatalogLayer> {
    let mut layer = CatalogLayer::default();
    for bundle in BUILTIN_BUNDLES {
        let entries = parse_translation_json(
            bundle.content,
            &format!("builtin:{}:{}", bundle.locale, bundle.namespace),
            true,
        )?;
        layer.insert_namespace(bundle.locale, bundle.namespace, entries);
    }
    Ok(layer)
}

fn load_overlay_layer(root: &Path) -> Result<OverlayLoadResult> {
    let mut result = OverlayLoadResult::default();

    if !root.exists() {
        return Ok(result);
    }

    let locale_dirs = fs::read_dir(root)
        .with_context(|| format!("读取 overlay 语言目录失败: {}", root.display()))?;
    for locale_entry in locale_dirs {
        let locale_entry = match locale_entry {
            Ok(value) => value,
            Err(error) => {
                result
                    .warnings
                    .push(format!("读取语言目录条目失败: {}", error));
                continue;
            }
        };

        let path = locale_entry.path();
        if !path.is_dir() {
            continue;
        }

        let locale = locale_entry.file_name().to_string_lossy().to_string();
        if let Err(error) = validate_locale_code(&locale) {
            result
                .warnings
                .push(format!("跳过非法 locale 目录 {}: {}", locale, error));
            continue;
        }

        let namespace_files = match fs::read_dir(&path) {
            Ok(value) => value,
            Err(error) => {
                result
                    .warnings
                    .push(format!("读取 locale 目录 {} 失败: {}", locale, error));
                continue;
            }
        };

        for namespace_entry in namespace_files {
            let namespace_entry = match namespace_entry {
                Ok(value) => value,
                Err(error) => {
                    result
                        .warnings
                        .push(format!("读取 namespace 条目失败: {}", error));
                    continue;
                }
            };

            let namespace_path = namespace_entry.path();
            if !namespace_path.is_file() {
                continue;
            }
            if namespace_path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }

            let namespace = match namespace_path.file_stem().and_then(|value| value.to_str()) {
                Some(value) => value.trim().to_string(),
                None => {
                    result.warnings.push(format!(
                        "跳过非法 namespace 文件: {}",
                        namespace_path.display()
                    ));
                    continue;
                }
            };
            if let Err(error) = validate_namespace(&namespace) {
                result.warnings.push(format!(
                    "跳过非法 namespace 文件 {}: {}",
                    namespace_path.display(),
                    error
                ));
                continue;
            }

            let content = match fs::read_to_string(&namespace_path) {
                Ok(value) => value,
                Err(error) => {
                    result.warnings.push(format!(
                        "读取 overlay 文件失败 {}: {}",
                        namespace_path.display(),
                        error
                    ));
                    continue;
                }
            };

            let entries = match parse_translation_json(
                &content,
                &format!("overlay:{}:{}", locale, namespace),
                true,
            ) {
                Ok(value) => value,
                Err(error) => {
                    result.warnings.push(format!(
                        "解析 overlay 文件失败 {}: {}",
                        namespace_path.display(),
                        error
                    ));
                    continue;
                }
            };

            result.layer.insert_namespace(&locale, &namespace, entries);
            result.loaded_files += 1;
        }
    }

    Ok(result)
}

fn parse_translation_json(
    content: &str,
    context: &str,
    strict_key: bool,
) -> Result<HashMap<String, String>> {
    let value: Value =
        serde_json::from_str(content).with_context(|| format!("{} JSON 解析失败", context))?;
    let object = value
        .as_object()
        .with_context(|| format!("{} 必须为 JSON 对象", context))?;

    let mut entries = HashMap::new();
    for (key, value) in object {
        if strict_key {
            validate_key(key)?;
        }
        let text = value
            .as_str()
            .with_context(|| format!("{} key={} 的值必须为字符串", context, key))?;
        entries.insert(key.clone(), text.to_string());
    }
    Ok(entries)
}

fn persist_overlay_namespace(
    overlay_root: &Path,
    locale: &str,
    namespace: &str,
    entries: &HashMap<String, String>,
    replace: bool,
) -> Result<()> {
    let locale_dir = overlay_root.join(locale);
    fs::create_dir_all(&locale_dir)
        .with_context(|| format!("创建 locale 目录失败: {}", locale_dir.display()))?;

    let file_path = locale_dir.join(format!("{}.json", namespace));
    if file_path.exists() && !replace {
        anyhow::bail!("导入失败: {} 已存在且 replace=false", file_path.display());
    }

    let temp_path = locale_dir.join(format!(".{}.json.tmp", namespace));
    let mut ordered = BTreeMap::new();
    for (key, value) in entries {
        ordered.insert(key.clone(), value.clone());
    }
    let serialized = serde_json::to_string_pretty(&ordered)
        .map(|value| format!("{}\n", value))
        .with_context(|| "序列化导入文件失败".to_string())?;
    fs::write(&temp_path, serialized)
        .with_context(|| format!("写入导入临时文件失败: {}", temp_path.display()))?;

    if let Err(error) = fs::rename(&temp_path, &file_path) {
        let _ = fs::remove_file(&temp_path);
        return Err(anyhow::Error::new(error).context(format!(
            "替换导入文件失败: {} -> {}",
            temp_path.display(),
            file_path.display()
        )));
    }

    Ok(())
}

fn collect_placeholder_warnings(
    catalog: &I18nCatalog,
    fallback_locale: &str,
    entries: &HashMap<String, String>,
) -> Vec<String> {
    let mut warnings = Vec::new();
    for (key, value) in entries {
        let Some(fallback_value) = catalog.lookup_in_locale(fallback_locale, key) else {
            continue;
        };

        let imported_placeholders = extract_placeholders(value);
        let fallback_placeholders = extract_placeholders(fallback_value);
        if imported_placeholders != fallback_placeholders {
            warnings.push(format!(
                "key={} 占位符不一致: imported={:?}, fallback={:?}",
                key, imported_placeholders, fallback_placeholders
            ));
        }
    }
    warnings
}

fn extract_placeholders(value: &str) -> BTreeSet<String> {
    let mut result = BTreeSet::new();

    // Legacy template format: {{name}}
    let mut rest = value;
    loop {
        let Some(start) = rest.find("{{") else {
            break;
        };
        let after_start = &rest[start + 2..];
        let Some(end) = after_start.find("}}") else {
            break;
        };
        if let Some(placeholder) = normalize_placeholder_name(&after_start[..end]) {
            result.insert(placeholder);
        }
        rest = &after_start[end + 2..];
    }

    // ICU format used by i18next-icu: {name} or {name, number}
    let bytes = value.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] != b'{' {
            index += 1;
            continue;
        }

        if index + 1 < bytes.len() && bytes[index + 1] == b'{' {
            index += 2;
            continue;
        }

        let start = index + 1;
        let Some(relative_end) = value[start..].find('}') else {
            break;
        };
        let end = start + relative_end;
        if let Some(placeholder) = normalize_placeholder_name(&value[start..end]) {
            result.insert(placeholder);
        }
        index = end + 1;
    }

    result
}

fn normalize_placeholder_name(raw: &str) -> Option<String> {
    let candidate = raw.split(',').next()?.trim();
    if candidate.is_empty() {
        return None;
    }
    if !candidate
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' || ch == '-')
    {
        return None;
    }
    Some(candidate.to_string())
}

fn validate_key(key: &str) -> Result<()> {
    anyhow::ensure!(!key.trim().is_empty(), "翻译 key 不能为空");
    anyhow::ensure!(
        key.chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-'),
        "翻译 key 非法: {}",
        key
    );
    Ok(())
}

fn validate_locale_code(locale: &str) -> Result<()> {
    let trimmed = locale.trim();
    let parts = trimmed.split('-').collect::<Vec<_>>();
    anyhow::ensure!(parts.len() == 2, "locale 格式非法: {}", locale);

    let language = parts[0];
    let region = parts[1];
    anyhow::ensure!(
        language.len() == 2 && language.chars().all(|ch| ch.is_ascii_alphabetic()),
        "locale language 非法: {}",
        locale
    );
    anyhow::ensure!(
        region.len() == 2 && region.chars().all(|ch| ch.is_ascii_alphabetic()),
        "locale region 非法: {}",
        locale
    );
    Ok(())
}

fn validate_namespace(namespace: &str) -> Result<()> {
    let value = namespace.trim();
    anyhow::ensure!(!value.is_empty(), "namespace 不能为空");
    anyhow::ensure!(
        value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'),
        "namespace 非法: {}",
        namespace
    );
    Ok(())
}
