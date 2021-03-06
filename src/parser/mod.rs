pub use ::{ Variable, Atom };
use ::std::fmt::{ Formatter, Display };

#[derive(Debug, Clone)]
pub struct Annotated<I>(pub I, pub Vec<()>);
impl<I> Annotated<I> {
    fn empty(inner: I) -> Self {
        Annotated(inner, Vec::new())
    }
}

#[derive(Debug, Clone)]
pub struct Module {
    pub name: Atom,
    pub declarations: Vec<FunctionName>,
    pub attributes: Vec<(Atom, Constant)>,
    pub definitions: Vec<FunctionDefinition>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct FunctionName {
    pub name: Atom,
    pub arity: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Integer {
    sign: bool,
    digits: String,
}
impl Integer {
    fn as_u32(&self) -> u32 {
        assert!(self.sign);
        self.digits.parse().unwrap()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AtomicLiteral {
    Integer(Integer),
    Float,
    Atom(Atom),
    Nil,
    Char(char),
    String(String),
}
impl Display for AtomicLiteral {
    fn fmt(&self, f: &mut Formatter) -> Result<(), ::std::fmt::Error> {
        match self {
            &AtomicLiteral::Integer(ref int) => write!(f, "{}{}", int.sign, int.digits),
            _ => write!(f, "unimpl"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Constant {
    Atomic(AtomicLiteral),
    Tuple(Vec<Constant>),
    List(Vec<Constant>, Box<Constant>),
}
impl Display for Constant {
    fn fmt(&self, f: &mut Formatter) -> Result<(), ::std::fmt::Error> {
        match self {
            &Constant::Atomic(ref lit) => write!(f, "{}", lit),
            _ => write!(f, "unimpl"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FunctionDefinition {
    pub name: Annotated<FunctionName>,
    pub fun: Annotated<Function>,
}

#[derive(Debug, Clone)]
pub enum SingleExpression {
    // Env reading
    FunctionName(FunctionName),
    ExternalFunctionName { module: Atom, name: FunctionName },
    Variable(Variable),

    // Control flow
    Let { vars: Vec<Annotated<Variable>>, val: Box<Expression>, body: Box<Expression> },
    Catch(Box<Expression>),
    Case { val: Box<Expression>, clauses: Vec<Annotated<CaseClause>> },
    Do(Box<Expression>, Box<Expression>),
    Try { body: Box<Expression>, then_vars: Vec<Annotated<Variable>>, then: Box<Expression>,
          catch_vars: Vec<Annotated<Variable>>, catch: Box<Expression> },
    Receive { clauses: Vec<Annotated<CaseClause>>, timeout_time: Box<Expression>,
              timeout_body: Box<Expression> },

    // Calling
    PrimOpCall(PrimOpCall),
    ApplyCall { fun: Box<Expression>, args: Vec<Expression> },
    InterModuleCall { module: Box<Expression>, name: Box<Expression>, args: Vec<Expression> },

    // Lambda creation
    Fun(Box<Function>),
    LetRec { funs: Vec<(FunctionName, Function)>, body: Box<Expression> },

    // Term constructors
    AtomicLiteral(AtomicLiteral),
    Tuple(Vec<Expression>),
    List { head: Vec<Expression>, tail: Box<Expression> },
    Map(Vec<(Expression, Expression)>, Option<Expression>),
    Binary(Vec<(Expression, Vec<Expression>)>),
}

#[derive(Debug, Clone)]
pub struct CaseClause {
    pub patterns: Vec<Annotated<Pattern>>,
    pub guard: Expression,
    pub body: Expression,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Wildcard,
    BindVar(Variable, Box<Annotated<Pattern>>),
    Atomic(AtomicLiteral),
    Binary(Vec<(Annotated<Pattern>, Vec<Annotated<SingleExpression>>)>),
    Tuple(Vec<Annotated<Pattern>>),
    List(Vec<Annotated<Pattern>>, Box<Annotated<Pattern>>),
    Map(Vec<(SingleExpression, Annotated<Pattern>)>),
}
impl Pattern {
    fn nil() -> Annotated<Pattern> {
        Annotated::empty(Pattern::Atomic(AtomicLiteral::Nil))
    }
}

pub type Expression = Annotated<Vec<Annotated<SingleExpression>>>;
impl Expression {
    fn nil() -> Self {
        Annotated::empty(vec![Annotated::empty(SingleExpression::AtomicLiteral(AtomicLiteral::Nil))])
    }
}

#[derive(Debug, Clone)]
pub struct Function {
    pub vars: Vec<Annotated<Variable>>,
    pub body: Expression,
}

#[derive(Debug, Clone)]
pub struct PrimOpCall {
    pub name: Atom,
    pub args: Vec<Expression>,
}

pub use self::core_parser::annotatedModule as annotated_module;
mod core_parser {
    include!(concat!(env!("OUT_DIR"), "/grammar.rs"));
}
