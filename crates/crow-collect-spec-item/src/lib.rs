// crow-collect-spec-item/src/lib.rs

pub mod errors;

use std::collections::HashMap;
use crow_ast::item::Attribute;
use crow_common::DefId;
use crow_hir::{Hir, item::{HirItem, HirItemKind}};
use errors::SpecialItemError;

/// Raw lang items — DefId level, before MIR lowering
pub struct LangItems {
    pub rc_alloc: Option<DefId>,
    pub rc_retain: Option<DefId>,
    pub rc_release: Option<DefId>,
    pub panic: Option<DefId>,
}

/// Intrinsic table — DefId level
pub struct IntrinsicTable {
    pub intrinsics: HashMap<DefId, String>,
}

impl IntrinsicTable {
    pub fn get(&self, def_id: DefId) -> Option<&str> {
        self.intrinsics.get(&def_id).map(|s| s.as_str())
    }

    pub fn is_intrinsic(&self, def_id: DefId) -> bool {
        self.intrinsics.contains_key(&def_id)
    }
}

/// Result of collection pass
pub struct SpecialItems {
    pub lang: LangItems,
    pub intrinsics: IntrinsicTable,
    pub errors: Vec<SpecialItemError>,
}

pub fn collect_special_items(hir: &Hir) -> SpecialItems {
    let mut lang = LangItems {
        rc_alloc: None,
        rc_retain: None,
        rc_release: None,
        panic: None,
    };
    let mut intrinsics = IntrinsicTable {
        intrinsics: HashMap::new(),
    };
    let mut errors = Vec::new();

    for item in hir.items.vec() {
        for attr in &item.attributes {
            match attr.name.as_str() {
                "intrinsic" => {
                    collect_intrinsic(item, attr, &mut intrinsics, &mut errors);
                }
                "lang_def" => {
                    collect_lang_item(item, attr, &mut lang, &mut errors);
                }
                _ => {}
            }
        }

        if let HirItemKind::Fun(f) = &item.kind {
            if f.body.is_none() && !item.has_attr("intrinsic") {
                errors.push(SpecialItemError::FunctionWithoutBody {
                    name: f.name.clone(),
                    src: item.span.0.clone().into(),
                    span: item.span.1.clone().into(),
                });
            }
        }
    }

    // Panic обязателен
    if lang.panic.is_none() {
        errors.push(SpecialItemError::MissingRequired {
            name: "panic".to_string(),
            hint: "@lang_def(\"panic\") fun panic(msg: string) -> ! { ... }".to_string(),
        });
    }

    // ADT требует полный GC
    let has_adt = hir.items.vec().iter().any(|item| {
        matches!(&item.kind, HirItemKind::Struct(_))
    });

    if has_adt {
        for (name, slot) in [
            ("rc_alloc", &lang.rc_alloc),
            ("rc_retain", &lang.rc_retain),
            ("rc_release", &lang.rc_release),
        ] {
            if slot.is_none() {
                errors.push(SpecialItemError::MissingRequired {
                    name: name.to_string(),
                    hint: format!("rec types require @lang_def(\"{}\")", name),
                });
            }
        }
    }

    SpecialItems { lang, intrinsics, errors }
}

fn collect_intrinsic(
    item: &HirItem,
    attr: &Attribute,
    table: &mut IntrinsicTable,
    errors: &mut Vec<SpecialItemError>,
) {
    let name = match &item.kind {
        HirItemKind::Fun(f) => &f.name,
        _ => return,
    };

    let intrinsic_name = match attr.args.first() {
        Some(n) => n.clone(),
        None => {
            errors.push(SpecialItemError::IntrinsicMissingArg {
                name: name.clone(),
                src: attr.span.0.clone().into(),
                span: attr.span.1.clone().into(),
            });
            return;
        }
    };

    if let HirItemKind::Fun(f) = &item.kind {
        if f.body.is_some() {
            errors.push(SpecialItemError::IntrinsicWithBody {
                name: name.clone(),
                src: item.span.0.clone().into(),
                span: item.span.1.clone().into(),
            });
        }
    }

    if table.intrinsics.contains_key(&item.def_id) {
        errors.push(SpecialItemError::DuplicateIntrinsic {
            name: name.clone(),
            src: item.span.0.clone().into(),
            span: attr.span.1.clone().into(),
        });
        return;
    }

    table.intrinsics.insert(item.def_id, intrinsic_name);
}

fn collect_lang_item(
    item: &HirItem,
    attr: &Attribute,
    lang: &mut LangItems,
    errors: &mut Vec<SpecialItemError>,
) {
    let name = match &item.kind {
        HirItemKind::Fun(f) => &f.name,
        _ => return,
    };

    let lang_name = match attr.args.first() {
        Some(n) => n.clone(),
        None => {
            errors.push(SpecialItemError::LangDefMissingArg {
                name: name.clone(),
                src: item.span.0.clone().into(),
                span: attr.span.1.clone().into(),
            });
            return;
        }
    };

    let slot = match lang_name.as_str() {
        "rc_alloc" => &mut lang.rc_alloc,
        "rc_retain" => &mut lang.rc_retain,
        "rc_release" => &mut lang.rc_release,
        "panic" => &mut lang.panic,
        other => {
            errors.push(SpecialItemError::UnknownLangItem {
                name: name.clone(),
                item: other.to_string(),
                src: item.span.0.clone().into(),
                span: attr.span.1.clone().into(),
            });
            return;
        }
    };

    if slot.is_some() {
        errors.push(SpecialItemError::DuplicateLangItem {
            name: lang_name,
            src: attr.span.0.clone().into(),
            span: attr.span.1.clone().into(),
        });
        return;
    }

    *slot = Some(item.def_id);
}