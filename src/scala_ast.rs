use itertools::{Itertools, Position};
use std::cell::RefCell;
use std::fmt::Write;

macro_rules! write {
    (
        $out:expr,
        $indent:expr,
        $format_str:expr,
        $($args:tt)*
    ) => {
        std::write!(
            $out,
            "{}{}",
            spaces($indent),
            std::format!($format_str, $($args)*)
        ).unwrap()
    };

    (
        $out:expr,
        $indent:expr,
        $format_str:expr
    ) => {
        std::write!(
            $out,
            "{}{}",
            spaces($indent),
            std::format!($format_str)
        ).unwrap()
    };
}

macro_rules! writeln {
    (
        $out:expr,
        $indent:expr,
        $format_str:expr,
        $($args:tt)*
    ) => {
        std::writeln!(
            $out,
            "{}{}",
            spaces($indent),
            std::format!($format_str, $($args)*)
        ).unwrap()
    };

    (
        $out:expr,
        $indent:expr,
        $format_str:expr
    ) => {
        std::writeln!(
            $out,
            "{}{}",
            spaces($indent),
            std::format!($format_str)
        ).unwrap()
    };
}

pub fn to_code<T: ToCode>(ast: T) -> String {
    let mut out = String::new();
    ast.to_code(&mut out, 0);
    out
}

pub trait ToCode {
    fn to_code(&self, out: &mut String, indent: usize);
}

#[derive(Debug)]
pub struct Ident {
    pub name: String,
}

impl Ident {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

impl ToCode for Ident {
    fn to_code(&self, out: &mut String, indent: usize) {
        if is_keyword(&self.name) {
            write!(out, indent, "`{}`", self.name)
        } else {
            write!(out, indent, "{}", self.name)
        }
    }
}

#[derive(Debug)]
pub enum Expr {
    Match {
        expr: Box<Expr>,
        clauses: Vec<MatchClause>,
    },
    StrLit {
        value: String,
        interpolate: bool,
    },
    Var {
        name: Ident,
    }
}

impl ToCode for Expr {
    fn to_code(&self, out: &mut String, indent: usize) {
        match self {
            Expr::Match { expr, clauses } => {
                expr.to_code(out, indent);
                writeln!(out, 0, " match {{");
                for clause in clauses {
                    clause.to_code(out, indent + 2);
                }
                writeln!(out, indent, "}}");
            }
            Expr::StrLit { value, interpolate } => {
                for line in value.split('\n').with_position() {
                    let start = if *interpolate { "s" } else { "" };

                    match line {
                        Position::Only(line) => {
                            write!(
                                out,
                                indent,
                                "{s}\"\"\"{value}\"\"\"",
                                s = start,
                                value = line
                            );
                        }
                        Position::First(line) => {
                            write!(
                                out,
                                indent,
                                "{s}\"\"\"{value}\n",
                                s = start,
                                value = line
                            );
                        }
                        Position::Middle(line) => {
                            write!(out, 0, "{}\n", line);
                        }
                        Position::Last(line) => {
                            write!(out, 0, "{}\"\"\"", line);
                        }
                    }
                }
            },
            Expr::Var { name } => {
                name.to_code(out, indent);
            }
        }
    }
}

#[derive(Debug)]
pub struct MatchClause {
    pub pattern: String,
    pub expr: Expr,
}

impl ToCode for MatchClause {
    fn to_code(&self, out: &mut String, indent: usize) {
        writeln!(out, indent, "case {} => {{", self.pattern);
        self.expr.to_code(out, indent + 2);
        writeln!(out, 0, "\n");
        writeln!(out, indent, "}}");
    }
}

fn is_keyword(ident: &str) -> bool {
    thread_local! {
        static SCALA_KEYWORDS: RefCell<Vec<&'static str>> = RefCell::new(
            vec![
                "abstract",
                "case",
                "catch",
                "class",
                "def",
                "do",
                "else",
                "extends",
                "false",
                "final",
                "finally",
                "for",
                "forSome",
                "if",
                "implicit",
                "import",
                "lazy",
                "match",
                "new",
                "null",
                "object",
                "override",
                "package",
                "private",
                "protected",
                "return",
                "sealed",
                "super",
                "this",
                "throw",
                "trait",
                "true",
                "try",
                "type",
                "val",
                "var",
                "while",
                "with",
                "yield"
            ]
        );
    }

    SCALA_KEYWORDS.with(|words| {
        let words = words.borrow();
        words.contains(&ident)
    })
}

#[derive(Debug)]
pub struct Param {
    pub name: Ident,
    pub ty: String,
}

impl ToCode for Param {
    fn to_code(&self, out: &mut String, indent: usize) {
        self.name.to_code(out, indent);
        write!(out, 0, ": {}", self.ty);
    }
}

#[derive(Debug)]
pub struct MethodDef {
    pub name: Ident,
    pub params: Vec<Param>,
    pub implicit_params: Vec<Param>,
    pub return_type: String,
    pub body: Expr,
    pub comment: Option<String>,
}

impl ToCode for MethodDef {
    fn to_code(&self, out: &mut String, indent: usize) {
        if let Some(comment) = &self.comment {
            writeln!(out, indent, "// {}", comment);
        }

        write!(out, indent, "def ");
        self.name.to_code(out, 0);

        if !self.params.is_empty() {
            write!(out, 0, "(");
            self.params.to_code(out, 0);
            write!(out, 0, ")");
        }

        if !self.implicit_params.is_empty() {
            write!(out, 0, "(implicit ");
            self.implicit_params.to_code(out, 0);
            write!(out, 0, ")");
        }

        writeln!(out, 0, ": {} = {{", self.return_type);
        self.body.to_code(out, indent + 2);
        writeln!(out, indent, "}}");
    }
}

impl ToCode for Vec<Param> {
    fn to_code(&self, out: &mut String, _indent: usize) {
        for param in self.iter().with_position() {
            match param {
                Position::Only(param) | Position::Last(param) => param.to_code(out, 0),
                Position::First(param) | Position::Middle(param) => {
                    param.to_code(out, 0);
                    write!(out, 0, ", ");
                }
            }
        }
    }
}

fn spaces(count: usize) -> String {
    std::iter::repeat(" ").take(count).collect()
}

#[derive(Debug)]
pub struct TopLevel {
    pub items: Vec<Item>,
}

impl ToCode for TopLevel {
    fn to_code(&self, out: &mut String, indent: usize) {
        for item in self.items.iter().with_position() {
            match item {
                Position::First(item) | Position::Middle(item) => {
                    item.to_code(out, indent);
                    write!(out, 0, "\n\n")
                }
                Position::Last(item) | Position::Only(item) => item.to_code(out, indent),
            }
        }
    }
}

#[derive(Debug)]
pub enum Item {
    Package {
        segments: Vec<Ident>,
    },
    Object {
        case: bool,
        name: String,
        items: Vec<Item>,
        methods: Vec<MethodDef>,
        super_type: Option<String>,
    },
    Trait {
        name: String,
        sealed: bool,
    },
}

impl ToCode for Item {
    fn to_code(&self, out: &mut String, indent: usize) {
        match self {
            Item::Package { segments } => {
                write!(out, indent, "package ");
                for segment in segments.iter().with_position() {
                    match segment {
                        Position::First(segment) | Position::Middle(segment) => {
                            segment.to_code(out, 0);
                            write!(out, 0, ".")
                        }
                        Position::Last(segment) | Position::Only(segment) => {
                            segment.to_code(out, 0)
                        }
                    }
                }
            }

            Item::Object {
                case,
                name,
                items,
                methods,
                super_type,
            } => {
                write!(out, indent, "");
                if *case {
                    write!(out, 0, "case ");
                }
                write!(out, 0, "object {}", name);

                if let Some(super_type) = super_type {
                    write!(out, 0, " extends {}", super_type);
                }

                if !items.is_empty() || !methods.is_empty() {
                    writeln!(out, 0, " {{");

                    for item in items.iter().with_position() {
                        match item {
                            Position::First(item) | Position::Middle(item) => {
                                item.to_code(out, indent + 2);
                                writeln!(out, 0, "\n")
                            }
                            Position::Last(item) | Position::Only(item) => {
                                item.to_code(out, indent + 2);
                            }
                        }
                    }

                    if !items.is_empty() && !methods.is_empty() {
                        write!(out, 0, "\n");
                    }

                    for method in methods {
                        method.to_code(out, indent + 2);
                    }

                    write!(out, 0, "\n");
                    write!(out, indent, "}}");
                }
            }

            Item::Trait { name, sealed } => {
                write!(out, indent, "");
                if *sealed {
                    write!(out, 0, "sealed ");
                }
                write!(out, 0, "trait {}", name);
            }
        }
    }
}
