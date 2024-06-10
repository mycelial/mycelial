//! experimental stateful queries
//!
use crate::Result;
use section::SectionError;
use sqlparser::ast::{BinaryOperator, CastKind, DataType, Expr, SetExpr, Statement, Value};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

#[derive(Debug)]
pub struct StatefulVariable {
    pub name: String,
    pub value: StatefulVariableValue,
}

#[derive(Debug, Clone)]
pub enum StatefulVariableValue {
    I64(i64),
}

impl TryFrom<&str> for StatefulVariableValue {
    type Error = SectionError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let v = match value.to_lowercase().as_str() {
            "i64" => StatefulVariableValue::I64(0),
            _ => Err(format!("unexpected type: {value}"))?,
        };
        Ok(v)
    }
}

// supports only select queries in for of
// select <rows> from <table> where <column> > $i::ty limit <value>
pub fn extract_variable_state(query: &str) -> Result<Option<(StatefulVariable, String)>> {
    let mut ast = Parser::parse_sql(&GenericDialect {}, query)?;
    if ast.len() != 1 {
        Err("only single query supported")?
    }
    let query = ast.first_mut().unwrap();
    let (selection, _limit) = match &mut *query {
        Statement::Query(ref mut q) if q.limit.is_some() => {
            let s = match &mut *q.body {
                SetExpr::Select(select) => select,
                _ => return Ok(None),
            };
            (s.selection.as_mut().unwrap(), q.limit.as_ref().unwrap())
        }
        _ => return Ok(None),
    };
    let (name, value): (String, StatefulVariableValue) = match selection {
        Expr::BinaryOp {
            op: BinaryOperator::Gt,
            ref mut right,
            ..
        } => {
            let mut new_right = Box::new(Expr::Value(Value::Placeholder("$1".into())));
            std::mem::swap(&mut new_right, right);
            match &*new_right {
                Expr::Cast {
                    kind: CastKind::DoubleColon,
                    expr,
                    data_type,
                    ..
                } => {
                    let name = match &**expr {
                        Expr::Value(Value::Placeholder(ident)) => {
                            ident.strip_prefix('$').unwrap().to_string()
                        }
                        _ => Err("expected placeholder value")?,
                    };
                    let ty = match data_type {
                        DataType::Custom(obj_name, _) if obj_name.0.len() == 1 => {
                            obj_name.0.first().unwrap().value.as_str().try_into()?
                        }
                        _ => Err("expected type")?,
                    };
                    (name, ty)
                }
                _ => Err("expected cast expression")?,
            }
        }
        _ => Err("unsupported query")?,
    };
    Ok(Some((StatefulVariable { name, value }, query.to_string())))
}
