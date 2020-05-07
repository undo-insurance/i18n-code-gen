use itertools::{Itertools, Position};
use std::cell::RefCell;
use std::fmt::Write;

macro_rules! write {
    ($($tt:tt)*) => {
        std::write!( $($tt)* ).unwrap()
    };
}

macro_rules! writeln {
    ($($tt:tt)*) => {
        std::writeln!( $($tt)* ).unwrap()
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
            write!(out, "{}`{}`", spaces(indent), self.name)
        } else {
            write!(out, "{}{}", spaces(indent), self.name)
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
}

impl ToCode for Expr {
    fn to_code(&self, out: &mut String, indent: usize) {
        match self {
            Expr::Match { expr, clauses } => {
                expr.to_code(out, indent);
                writeln!(out, " match {{");
                for clause in clauses {
                    clause.to_code(out, indent + 2);
                }
                writeln!(out, "{}}}", spaces(indent));
            }
            Expr::StrLit { value, interpolate } => {
                for line in value.split('\n').with_position() {
                    let start = if *interpolate { "s" } else { "" };

                    match line {
                        Position::Only(line) => {
                            write!(
                                out,
                                "{indent}{s}\"\"\"{value}\"\"\"",
                                indent = spaces(indent),
                                s = start,
                                value = line
                            );
                        }
                        Position::First(line) => {
                            write!(
                                out,
                                "{indent}{s}\"\"\"{value}\n",
                                indent = spaces(indent),
                                s = start,
                                value = line
                            );
                        }
                        Position::Middle(line) => {
                            write!(out, "{}\n", line);
                        }
                        Position::Last(line) => {
                            write!(out, "{}\"\"\"", line);
                        }
                    }
                }
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
        writeln!(out, "{}case {} => {{", spaces(indent), self.pattern);
        self.expr.to_code(out, indent + 2);
        writeln!(out, "\n{}}}", spaces(indent));
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
        write!(out, ": {}", self.ty);
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
            writeln!(out, "{}// {}", spaces(indent), comment);
        }

        write!(out, "{}def ", spaces(indent));
        self.name.to_code(out, 0);

        if !self.params.is_empty() {
            write!(out, "(");
            self.params.to_code(out, 0);
            write!(out, ")");
        }

        if !self.implicit_params.is_empty() {
            write!(out, "(implicit ");
            self.implicit_params.to_code(out, 0);
            write!(out, ")");
        }

        writeln!(out, ": {} = {{", self.return_type);
        self.body.to_code(out, indent + 2);
        writeln!(out, "{}}}", spaces(indent));
    }
}

impl ToCode for Vec<Param> {
    fn to_code(&self, out: &mut String, _indent: usize) {
        for param in self.iter().with_position() {
            match param {
                Position::Only(param) | Position::Last(param) => param.to_code(out, 0),
                Position::First(param) | Position::Middle(param) => {
                    param.to_code(out, 0);
                    write!(out, ", ");
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
                    write!(out, "\n\n")
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
                write!(out, "{}package ", spaces(indent));
                for segment in segments.iter().with_position() {
                    match segment {
                        Position::First(segment) | Position::Middle(segment) => {
                            segment.to_code(out, 0);
                            write!(out, ".")
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
                super_type
            } => {
                write!(out, "{}", spaces(indent));
                if *case {
                    write!(out, "case ");
                }
                write!(out, "object {}", name);

                if let Some(super_type) = super_type {
                    write!(out, " extends {}", super_type);
                }

                if !items.is_empty() || !methods.is_empty() {
                    writeln!(out, " {{");
                    for item in items.iter().with_position() {
                        match item {
                            Position::First(item) | Position::Middle(item) => {
                                item.to_code(out, indent + 2);
                                write!(out, "\n")
                            }
                            Position::Last(item) | Position::Only(item) => {
                                item.to_code(out, indent + 2);
                            }
                        }
                    }

                    if !items.is_empty() && !methods.is_empty() {
                        write!(out, "\n");
                    }

                    for method in methods {
                        method.to_code(out, indent + 2);
                    }

                    write!(out, "{}\n}}", spaces(indent));
                }
            }

            Item::Trait { name, sealed } => {
                write!(out, "{}", spaces(indent));
                if *sealed {
                    write!(out, "sealed ");
                }
                write!(out, "trait {}", name);
            }
        }
    }
}
