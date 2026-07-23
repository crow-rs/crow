use crow_ast::atom::Publicity;
use crow_hir::{Hir, item::HirItemKind};
use crow_mod_codec_ty::{ExportedConst, ExportedFn, ExportedType, ExportedTypeKind, ModuleTop};
use crow_tycheck::typeck::TypeckOutput;

pub fn serialize_module_info(
    module_name: &str,
    hir: &Hir,
    typeck: &TypeckOutput,
) -> ModuleTop {
    let mut functions = Vec::new();
    let mut types = Vec::new();
    let mut constants = Vec::new();
    for item in hir.items.vec() {
        if item.publicity != Publicity::Pub { continue; }

        match &item.kind {
            HirItemKind::Fun(f) => {
                if let Some(sig) = typeck.fn_sigs.get(&item.def_id) {
                    functions.push(ExportedFn {
                        name: f.name.clone(),
                        params: f.params.iter()
                            .zip(sig.params.iter())
                            .map(|(p, ty)| (p.name.clone(), format!("{}", ty)))
                            .collect(),
                        ret: format!("{}", sig.ret),
                        is_native: false,
                    });
                }
            }
            HirItemKind::Native(n) => {
                if let Some(sig) = typeck.fn_sigs.get(&item.def_id) {
                    functions.push(ExportedFn {
                        name: n.name.clone(),
                        params: n.params.iter()
                            .zip(sig.params.iter())
                            .map(|(p, ty)| (p.name.clone(), format!("{}", ty)))
                            .collect(),
                        ret: format!("{}", sig.ret),
                        is_native: true,
                    });
                }
            }
            HirItemKind::Struct(s) => {
                if let Some(fields) = typeck.field_tys.get(&item.def_id) {
                    types.push(ExportedType {
                        name: s.name.clone(),
                        kind: ExportedTypeKind::Struct {
                            fields: fields.iter()
                                .map(|(name, ty)| (name.clone(), format!("{}", ty)))
                                .collect(),
                        },
                    });
                }
            }
            HirItemKind::Const(c) => {
                if let Some(ty) = typeck.constants.get(&item.def_id) {
                    constants.push(ExportedConst {
                        name: c.name.clone(),
                        ty: format!("{}", ty) 
                    });
                }
            }
            _ => ()
        }
    }

    ModuleTop {
        module_name: module_name.to_string(),
        functions,
        types,
        constants: constants,
    }
}