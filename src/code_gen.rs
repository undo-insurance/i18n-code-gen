// def generateCode(keys: List[Key]): Try[String] = {

use crate::lokalise_client::{Key, KeyName, LokaliseClient, Project, Translation};
use crate::scala_ast::*;
use anyhow::{Error, Result};

pub fn generate_code(keys: Vec<Key>) -> Result<String> {
    CodeGen::default().gen(keys)
}

#[derive(Debug, Default)]
struct CodeGen {
    ident: usize,
}

impl CodeGen {
    fn gen(&self, keys: Vec<Key>) -> Result<String> {
        let items = vec![
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
        ];

        let ast: TopLevel = TopLevel { items };
        Ok(to_code(ast))
    }
}
