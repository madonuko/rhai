use crate::def_package;
use crate::plugin::*;
use crate::types::dynamic::Tag;
use crate::{Dynamic, RhaiResultOf, ERR, INT};
#[cfg(feature = "no_std")]
use std::prelude::v1::*;

def_package! {
    /// Package of core language features.
    crate::LanguageCorePackage => |lib| {
        lib.standard = true;

        combine_with_exported_module!(lib, "language_core", core_functions);
    }
}

#[export_module]
mod core_functions {
    #[rhai_fn(name = "!")]
    pub fn not(x: bool) -> bool {
        !x
    }
    #[rhai_fn(name = "tag", get = "tag", pure)]
    pub fn get_tag(value: &mut Dynamic) -> INT {
        value.tag() as INT
    }
    #[rhai_fn(name = "set_tag", set = "tag", return_raw)]
    pub fn set_tag(value: &mut Dynamic, tag: INT) -> RhaiResultOf<()> {
        if tag < Tag::MIN as INT {
            Err(ERR::ErrorArithmetic(
                format!(
                    "{} is too small to fit into a tag (must be between {} and {})",
                    tag,
                    Tag::MIN,
                    Tag::MAX
                ),
                Position::NONE,
            )
            .into())
        } else if tag > Tag::MAX as INT {
            Err(ERR::ErrorArithmetic(
                format!(
                    "{} is too large to fit into a tag (must be between {} and {})",
                    tag,
                    Tag::MIN,
                    Tag::MAX
                ),
                Position::NONE,
            )
            .into())
        } else {
            value.set_tag(tag as Tag);
            Ok(())
        }
    }

    #[cfg(not(feature = "no_function"))]
    #[cfg(not(feature = "no_index"))]
    #[cfg(not(feature = "no_object"))]
    pub fn get_fn_metadata_list(ctx: NativeCallContext) -> crate::Array {
        collect_fn_metadata(ctx)
    }
}

#[cfg(not(feature = "no_function"))]
#[cfg(not(feature = "no_index"))]
#[cfg(not(feature = "no_object"))]
fn collect_fn_metadata(ctx: NativeCallContext) -> crate::Array {
    use crate::{ast::ScriptFnDef, Array, Identifier, Map};
    use std::collections::BTreeSet;

    // Create a metadata record for a function.
    fn make_metadata(
        dict: &BTreeSet<Identifier>,
        namespace: Option<Identifier>,
        func: &ScriptFnDef,
    ) -> Map {
        const DICT: &str = "key exists";

        let mut map = Map::new();

        if let Some(ns) = namespace {
            map.insert(dict.get("namespace").expect(DICT).clone(), ns.into());
        }
        map.insert(
            dict.get("name").expect(DICT).clone(),
            func.name.clone().into(),
        );
        map.insert(
            dict.get("access").expect(DICT).clone(),
            match func.access {
                FnAccess::Public => dict.get("public").expect(DICT).clone(),
                FnAccess::Private => dict.get("private").expect(DICT).clone(),
            }
            .into(),
        );
        map.insert(
            dict.get("is_anonymous").expect(DICT).clone(),
            func.name.starts_with(crate::engine::FN_ANONYMOUS).into(),
        );
        map.insert(
            dict.get("params").expect(DICT).clone(),
            func.params
                .iter()
                .cloned()
                .map(Into::into)
                .collect::<Array>()
                .into(),
        );

        map
    }

    // Intern strings
    let dict: BTreeSet<Identifier> = [
        "namespace",
        "name",
        "access",
        "public",
        "private",
        "is_anonymous",
        "params",
    ]
    .iter()
    .map(|&s| s.into())
    .collect();

    let mut _list = ctx.iter_namespaces().flat_map(Module::iter_script_fn).fold(
        Array::new(),
        |mut list, (_, _, _, _, f)| {
            list.push(make_metadata(&dict, None, f).into());
            list
        },
    );

    #[cfg(not(feature = "no_module"))]
    {
        // Recursively scan modules for script-defined functions.
        fn scan_module(
            list: &mut Array,
            dict: &BTreeSet<Identifier>,
            namespace: Identifier,
            module: &Module,
        ) {
            module.iter_script_fn().for_each(|(_, _, _, _, f)| {
                list.push(make_metadata(dict, Some(namespace.clone()), f).into())
            });
            module.iter_sub_modules().for_each(|(ns, m)| {
                let ns = format!(
                    "{}{}{}",
                    namespace,
                    crate::tokenizer::Token::DoubleColon.literal_syntax(),
                    ns
                );
                scan_module(list, dict, ns.into(), m.as_ref())
            });
        }

        ctx.iter_imports_raw()
            .for_each(|(ns, m)| scan_module(&mut _list, &dict, ns.clone(), m.as_ref()));
    }

    _list
}