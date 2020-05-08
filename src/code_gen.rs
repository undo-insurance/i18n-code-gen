// def generateCode(keys: List[Key]): Try[String] = {

use crate::lokalise_client::{Key, KeyName, LokaliseClient, Project, Translation};
use crate::scala_ast::*;
use anyhow::{Error, Result};
use heck::CamelCase;
use std::collections::HashSet;

pub fn generate_code(keys: Vec<Key>) -> Result<String> {
    let mut items = hardcoded_items();

    items.extend(vec![
        Item::Trait {
            name: "Locale".to_string(),
            sealed: true,
        },
        Item::Object {
            case: false,
            name: "Locale".to_string(),
            items: locale_enum_variants(keys),
            methods: vec![],
            super_type: None,
        },
    ]);

    items.push(Item::Object {
        case: false,
        name: "I18n".to_string(),
        items: vec![],
        methods: vec![],
        super_type: None,
    });

    let ast = TopLevel { items };
    Ok(to_code(ast))
}

fn locale_enum_variants(keys: Vec<Key>) -> Vec<Item> {
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

fn find_locales(keys: Vec<Key>) -> HashSet<String> {
    keys.into_iter()
        .flat_map(|key| key.translations)
        .map(|translation| translation.language_iso)
        .collect()
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
