use crate::lokalise_client::Project;
use crate::{lokalise_client::Key, scala_ast::*};
use anyhow::{Error, Result};
use heck::{CamelCase, MixedCase, TitleCase};
use regex::Regex;
use serde::Deserialize;
use std::{collections::HashSet, str::FromStr};

pub fn generate_code(projects: Vec<(Project, Vec<Key>)>) -> Result<String> {
    let mut items = Vec::new();

    items.push(Item::Comment(Comment::new("format: off")));

    items.extend(hardcoded_items());

    let all_keys = projects
        .iter()
        .flat_map(|(_, keys)| keys)
        .collect::<Vec<_>>();

    items.extend(vec![
        Item::Trait {
            name: "Locale".to_string(),
            sealed: true,
        },
        Item::Object {
            case: false,
            name: "Locale".to_string(),
            items: locale_enum_variants(&all_keys),
            methods: vec![],
            super_type: None,
        },
    ]);

    let items_inside_i18n_obj = projects
        .into_iter()
        .map(|(project, keys)| {
            let methods = translation_methods(&keys)?;
            Ok(Item::Object {
                case: false,
                name: project.name.to_mixed_case(),
                items: vec![],
                methods,
                super_type: None,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    items.extend(vec![Item::Object {
        case: false,
        name: "I18n".to_string(),
        items: items_inside_i18n_obj,
        methods: vec![],
        super_type: None,
    }]);

    items.push(Item::Comment(Comment::new("format: on")));

    let ast = TopLevel { items };
    Ok(to_code(ast))
}

fn locale_enum_variants(keys: &[&Key]) -> Vec<Item> {
    let locales = find_locales(keys);

    locales
        .into_iter()
        .map(|locale| Item::Object {
            case: false,
            name: locale.to_camel_case(),
            items: vec![],
            methods: vec![],
            super_type: Some("Locale".to_string()),
        })
        .collect()
}

fn find_locales<'a>(keys: &[&'a Key]) -> Vec<&'a str> {
    let mut names = keys
        .iter()
        .flat_map(|key| &key.translations)
        .map(|translation| translation.language_iso.as_str())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn translation_methods(keys: &[Key]) -> Result<Vec<MethodDef>> {
    keys.iter().map(|key| translation_method(key)).collect()
}

fn translation_method(key: &Key) -> Result<MethodDef> {
    if !key.key_name.all_same() {
        return Err(Error::msg(format!(
            "Key {:?} don't have identical key names for each platform. Dunno which one to pick.",
            key
        )));
    }

    if key.is_plural {
        translation_method_with_cardinality(key)
    } else {
        translation_method_without_cardinality(key)
    }
}

fn translation_method_with_cardinality(key: &Key) -> Result<MethodDef> {
    let (placeholders, mut method_params) = build_method_params(key)?;
    method_params.push(Param {
        name: Ident::new("cardinality"),
        ty: "Cardinality".to_string(),
    });

    let locale_match_clauses = key
        .translations
        .iter()
        .map(|translation| -> Result<_> {
            let cases =
                serde_json::from_str::<TranslationWithCardinality>(&translation.translation)?;

            let singular_value =
                build_translated_value_with_interpolations(&cases.one, &placeholders);
            let plural_value =
                build_translated_value_with_interpolations(&cases.other, &placeholders);

            let cardinality_match_clauses = vec![
                MatchClause {
                    pattern: "Cardinality.Singular".to_string(),
                    expr: singular_value,
                },
                MatchClause {
                    pattern: "Cardinality.Plural".to_string(),
                    expr: plural_value,
                },
            ];

            Ok(MatchClause {
                pattern: format!("Locale.{}", translation.language_iso.to_title_case()),
                expr: Expr::Match {
                    expr: Box::new(Expr::Var {
                        name: Ident::new("cardinality"),
                    }),
                    clauses: cardinality_match_clauses,
                },
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let name = Ident::new(key.key_name.ios.to_mixed_case());
    Ok(MethodDef {
        name,
        params: method_params,
        implicit_params: vec![Param {
            name: Ident::new("locale"),
            ty: "Locale".to_string(),
        }],
        body: Expr::Match {
            expr: Box::new(Expr::Var {
                name: Ident::new("locale"),
            }),
            clauses: locale_match_clauses,
        },
        return_type: "String".to_string(),
        comment: Some(Comment::new(&key.key_name.ios)),
    })
}

fn translation_method_without_cardinality(key: &Key) -> Result<MethodDef> {
    let (placeholders, method_params) = build_method_params(key)?;

    let locale_match_clauses = key
        .translations
        .iter()
        .map(|translation| {
            let value =
                build_translated_value_with_interpolations(&translation.translation, &placeholders);

            MatchClause {
                pattern: format!("Locale.{}", translation.language_iso.to_title_case()),
                expr: value,
            }
        })
        .collect::<Vec<_>>();

    let name = Ident::new(key.key_name.ios.to_mixed_case());

    Ok(MethodDef {
        name,
        params: method_params,
        implicit_params: vec![Param {
            name: Ident::new("locale"),
            ty: "Locale".to_string(),
        }],
        body: Expr::Match {
            expr: Box::new(Expr::Var {
                name: Ident::new("locale"),
            }),
            clauses: locale_match_clauses,
        },
        return_type: "String".to_string(),
        comment: Some(Comment::new(&key.key_name.ios)),
    })
}

#[derive(Deserialize)]
struct TranslationWithCardinality {
    one: String,
    other: String,
}

fn build_method_params(key: &Key) -> Result<(Vec<Placeholder>, Vec<Param>)> {
    let mut placeholders = key
        .translations
        .iter()
        .map(|k| &k.translation)
        .map(|t| find_placeholders(t))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    placeholders.sort_unstable_by_key(|p| p.name.clone());

    let method_params = placeholders
        .iter()
        .map(|placeholder| {
            let ty = match placeholder.kind {
                PlaceholderKind::String => "String",
                PlaceholderKind::Integer => "Int",
            }
            .to_string();
            Param {
                name: Ident::new(&placeholder.name),
                ty,
            }
        })
        .collect::<Vec<_>>();

    Ok((placeholders, method_params))
}

#[derive(Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct Placeholder {
    name: String,
    kind: PlaceholderKind,
    matched: String,
}

#[derive(Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
enum PlaceholderKind {
    String,
    Integer,
}

impl FromStr for PlaceholderKind {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "s" => Ok(PlaceholderKind::String),
            "i" => Ok(PlaceholderKind::Integer),
            _ => Err(Error::msg(format!("Unsupported placeholder kind: {:?}", s))),
        }
    }
}

pub fn find_placeholders(s: &str) -> Result<Vec<Placeholder>> {
    lazy_static::lazy_static! {
        static ref RE: Regex = Regex::new(
            r#"\[%([si]):([^\]]+)\]"#
        ).unwrap();
    }

    RE.captures_iter(s)
        .map(|caps| -> Result<_> {
            let raw_kind = &caps[1];
            let kind = raw_kind.parse::<PlaceholderKind>()?;
            let name = caps
                .get(2)
                .ok_or_else(|| Error::msg("placeholder regex didn't match"))?
                .as_str()
                .to_mixed_case();

            let matched = caps
                .get(0)
                .ok_or_else(|| Error::msg("placeholder regex didn't match"))?
                .as_str()
                .to_string();

            Ok(Placeholder {
                name,
                kind,
                matched,
            })
        })
        .collect::<Result<Vec<_>>>()
}

fn build_translated_value_with_interpolations(
    translation: &str,
    placeholders: &[Placeholder],
) -> Expr {
    let mut translation = translation.to_string();
    for placeholder in placeholders {
        translation =
            translation.replace(&placeholder.matched, &format!("${{{}}}", placeholder.name));
    }

    Expr::StrLit {
        value: translation,
        interpolate: !placeholders.is_empty(),
    }
}

fn hardcoded_items() -> Vec<Item> {
    vec![
        Item::Package {
            segments: vec![Ident::new("dk"), Ident::new("undo"), Ident::new("i18n")],
        },
        Item::Trait {
            name: "Cardinality".to_string(),
            sealed: true,
        },
        Item::Object {
            name: "Cardinality".to_string(),
            case: false,
            methods: vec![],
            items: vec![
                Item::Object {
                    name: "Singular".to_string(),
                    case: true,
                    methods: vec![],
                    items: vec![],
                    super_type: Some("Cardinality".to_string()),
                },
                Item::Object {
                    name: "Plural".to_string(),
                    case: true,
                    methods: vec![],
                    items: vec![],
                    super_type: Some("Cardinality".to_string()),
                },
            ],
            super_type: None,
        },
    ]
}
