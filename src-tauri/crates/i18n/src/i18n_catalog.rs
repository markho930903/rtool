use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::Path;
use std::sync::{OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard};

struct BuiltinBundle {
    locale: &'static str,
    namespace: &'static str,
    content: &'static str,
}

const BUILTIN_BUNDLES: &[BuiltinBundle] = &[
    BuiltinBundle {
        locale: "zh-CN",
        namespace: "common",
        content: include_str!("../../../../i18n/source/zh-CN/common.json"),
    },
    BuiltinBundle {
        locale: "zh-CN",
        namespace: "layout",
        content: include_str!("../../../../i18n/source/zh-CN/layout.json"),
    },
    BuiltinBundle {
        locale: "zh-CN",
        namespace: "home",
        content: include_str!("../../../../i18n/source/zh-CN/home.json"),
    },
    BuiltinBundle {
        locale: "zh-CN",
        namespace: "tools",
        content: include_str!("../../../../i18n/source/zh-CN/tools.json"),
    },
    BuiltinBundle {
        locale: "zh-CN",
        namespace: "transfer",
        content: include_str!("../../../../i18n/source/zh-CN/transfer.json"),
    },
    BuiltinBundle {
        locale: "zh-CN",
        namespace: "logs",
        content: include_str!("../../../../i18n/source/zh-CN/logs.json"),
    },
    BuiltinBundle {
        locale: "zh-CN",
        namespace: "settings",
        content: include_str!("../../../../i18n/source/zh-CN/settings.json"),
    },
    BuiltinBundle {
        locale: "zh-CN",
        namespace: "clipboard",
        content: include_str!("../../../../i18n/source/zh-CN/clipboard.json"),
    },
    BuiltinBundle {
        locale: "zh-CN",
        namespace: "palette",
        content: include_str!("../../../../i18n/source/zh-CN/palette.json"),
    },
    BuiltinBundle {
        locale: "zh-CN",
        namespace: "not_found",
        content: include_str!("../../../../i18n/source/zh-CN/not_found.json"),
    },
    BuiltinBundle {
        locale: "zh-CN",
        namespace: "native",
        content: include_str!("../../../../i18n/source/zh-CN/native.json"),
    },
    BuiltinBundle {
        locale: "en-US",
        namespace: "common",
        content: include_str!("../../../../i18n/source/en-US/common.json"),
    },
    BuiltinBundle {
        locale: "en-US",
        namespace: "layout",
        content: include_str!("../../../../i18n/source/en-US/layout.json"),
    },
    BuiltinBundle {
        locale: "en-US",
        namespace: "home",
        content: include_str!("../../../../i18n/source/en-US/home.json"),
    },
    BuiltinBundle {
        locale: "en-US",
        namespace: "tools",
        content: include_str!("../../../../i18n/source/en-US/tools.json"),
    },
    BuiltinBundle {
        locale: "en-US",
        namespace: "transfer",
        content: include_str!("../../../../i18n/source/en-US/transfer.json"),
    },
    BuiltinBundle {
        locale: "en-US",
        namespace: "logs",
        content: include_str!("../../../../i18n/source/en-US/logs.json"),
    },
    BuiltinBundle {
        locale: "en-US",
        namespace: "settings",
        content: include_str!("../../../../i18n/source/en-US/settings.json"),
    },
    BuiltinBundle {
        locale: "en-US",
        namespace: "clipboard",
        content: include_str!("../../../../i18n/source/en-US/clipboard.json"),
    },
    BuiltinBundle {
        locale: "en-US",
        namespace: "palette",
        content: include_str!("../../../../i18n/source/en-US/palette.json"),
    },
    BuiltinBundle {
        locale: "en-US",
        namespace: "not_found",
        content: include_str!("../../../../i18n/source/en-US/not_found.json"),
    },
    BuiltinBundle {
        locale: "en-US",
        namespace: "native",
        content: include_str!("../../../../i18n/source/en-US/native.json"),
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

}

#[derive(Debug, Clone)]
struct I18nCatalog {
    builtin: CatalogLayer,
    overlay: CatalogLayer,
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

fn load_builtin_layer() -> Result<CatalogLayer> {
    let mut layer = CatalogLayer::default();
    for bundle in BUILTIN_BUNDLES {
        let entries = parse_translation_json(
            bundle.content,
            &format!("builtin:{}:{}", bundle.locale, bundle.namespace),
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
) -> Result<HashMap<String, String>> {
    let value: Value =
        serde_json::from_str(content).with_context(|| format!("{} JSON 解析失败", context))?;
    let object = value
        .as_object()
        .with_context(|| format!("{} 必须为 JSON 对象", context))?;

    let mut entries = HashMap::new();
    for (key, value) in object {
        validate_key(key)?;
        let text = value
            .as_str()
            .with_context(|| format!("{} key={} 的值必须为字符串", context, key))?;
        entries.insert(key.clone(), text.to_string());
    }
    Ok(entries)
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
