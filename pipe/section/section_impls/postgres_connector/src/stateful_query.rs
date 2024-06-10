//! experimental stateful queries
//!
use crate::Result;
use section::SectionError;
use sqlparser::ast::{BinaryOperator, CastKind, DataType, Expr, SetExpr, Statement, Value};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

#[derive(Debug, Clone, PartialEq)]
pub struct StatefulVariable {
    pub name: String,
    pub value: StatefulVariableValue,
    pub placeholder_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug)]
pub struct StatefulVariableParser {
    statement: Option<Statement>,
    placeholder_count: usize,
    var: Option<StatefulVariable>,
}

impl StatefulVariableParser {
    pub fn new(query: &str) -> Result<Self> {
        let mut statements = Parser::parse_sql(&GenericDialect {}, query)?;
        if statements.len() != 1 {
            Err("expected query with single statement")?
        };
        Ok(Self {
            statement: statements.pop(),
            placeholder_count: 0,
            var: None,
        })
    }

    pub fn parse(mut self) -> Result<Option<(StatefulVariable, String)>> {
        let mut statement = match self.statement.take() {
            Some(statement) => statement,
            None => Err("no statement found")?,
        };
        let selection = match &mut statement {
            Statement::Query(ref mut q) => {
                let s = match &mut *q.body {
                    SetExpr::Select(select) => select,
                    _ => return Ok(None),
                };
                s.selection.as_mut().unwrap()
            }
            _ => return Ok(None),
        };
        match selection {
            Expr::BinaryOp { left, op, right } => self.parse_binary_op(left, &*op, right)?,
            Expr::Between {
                negated, low, high, ..
            } if !(*negated) => self.parse_between(low, high)?,
            _ => Err("unsupported selection")?,
        };
        if let Some(var) = self.var.take() {
            Ok(Some((var, statement.to_string())))
        } else {
            Ok(None)
        }
    }

    fn next_placeholder(&mut self) -> usize {
        self.placeholder_count += 1;
        self.placeholder_count
    }

    // parse binary operation and rewrite placeholders
    fn parse_binary_op(
        &mut self,
        left: &mut Expr,
        _op: &BinaryOperator,
        right: &mut Expr,
    ) -> Result<()> {
        match left {
            Expr::Cast { .. } => {
                let mut new_left = self.new_placeholder();
                std::mem::swap(&mut *new_left, left);
                self.parse_cast(&new_left)?
            }
            Expr::BinaryOp { left, op, right } => self.parse_binary_op(left, &*op, right)?,
            _ => (),
        };

        match right {
            Expr::Cast { .. } => {
                let mut new_right = self.new_placeholder();
                std::mem::swap(&mut *new_right, right);
                self.parse_cast(&new_right)?
            }
            Expr::BinaryOp { left, op, right } => self.parse_binary_op(left, &*op, right)?,
            _ => (),
        }
        Ok(())
    }

    fn new_placeholder(&mut self) -> Box<Expr> {
        Box::new(Expr::Value(Value::Placeholder(format!(
            "${}",
            self.next_placeholder()
        ))))
    }

    fn parse_cast(&mut self, cast: &Expr) -> Result<()> {
        let (kind, expr, data_type) = if let Expr::Cast {
            kind,
            expr,
            data_type,
            ..
        } = cast
        {
            (kind, expr, data_type)
        } else {
            return Err(format!("expected cast, got: {cast:?}"))?;
        };
        match kind {
            CastKind::DoubleColon => (),
            _ => Err(format!("unsupported cast kind: {:?}", kind))?,
        };
        let name = match &**expr {
            Expr::Value(Value::Placeholder(ident)) => ident.strip_prefix('$').unwrap().to_string(),
            _ => Err("expected placeholder value")?,
        };
        let value = match data_type {
            DataType::Custom(obj_name, _) if obj_name.0.len() == 1 => {
                obj_name.0.first().unwrap().value.as_str().try_into()?
            }
            _ => Err("expected type")?,
        };
        if let Some(var) = self.var.as_ref() {
            if (var.name != name) || (var.value != value) {
                Err("stateful variable already present and it has different name")?;
            }
        }
        self.var = Some(StatefulVariable {
            name,
            value,
            placeholder_count: self.placeholder_count,
        });
        Ok(())
    }

    // parse and rewrite between query
    fn parse_between(&mut self, low: &mut Expr, high: &mut Expr) -> Result<()> {
        match low {
            Expr::Cast { .. } => {
                let mut new_low = self.new_placeholder();
                std::mem::swap(&mut *new_low, low);
                self.parse_cast(&new_low)?
            }
            Expr::BinaryOp { left, op, right } => self.parse_binary_op(left, &*op, right)?,
            _ => (),
        };
        if self.var.is_none() {
            Err("malformed between statement")?
        }
        let low = self.var.take().unwrap();

        match high {
            Expr::Cast { .. } => {
                let mut new_high = self.new_placeholder();
                std::mem::swap(&mut *new_high, high);
                self.parse_cast(&new_high)?
            }
            Expr::BinaryOp { left, op, right } => self.parse_binary_op(left, &*op, right)?,
            _ => (),
        };
        if self.var.is_none() {
            Err("malformed between statement")?
        }
        let high = self.var.take().unwrap();

        if low.name != high.name {
            Err("name differs between high and low")?
        };

        if low.value != high.value {
            Err("values are different")?
        };
        self.var = Some(high);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_gt_limit_query() {
        let query = "select * from test where id > $id::i64 limit 10000";
        let parser = StatefulVariableParser::new(query).unwrap();
        let var = parser.parse();
        assert!(
            matches!(var, Ok(Some(_))),
            "failed to parse query: {:?}",
            var
        );
        let (var, new_query) = var.unwrap().unwrap();
        assert_eq!(new_query, "SELECT * FROM test WHERE id > $1 LIMIT 10000");
        assert_eq!(
            var,
            StatefulVariable {
                name: "id".into(),
                value: StatefulVariableValue::I64(0),
                placeholder_count: 1,
            }
        )
    }

    #[test]
    fn test_between_query() {
        let query = "select * from test where id between $id::i64 and $id::i64 + 128000";
        let parser = StatefulVariableParser::new(query).unwrap();
        let var = parser.parse();
        assert!(
            matches!(var, Ok(Some(_))),
            "failed to parse query: {:?}",
            var
        );
        let (var, new_query) = var.unwrap().unwrap();
        assert_eq!(
            new_query,
            "SELECT * FROM test WHERE id BETWEEN $1 AND $2 + 128000"
        );
        assert_eq!(
            var,
            StatefulVariable {
                name: "id".into(),
                value: StatefulVariableValue::I64(0),
                placeholder_count: 2
            }
        )
    }

    #[test]
    fn test_range_query() {
        let query = "select * from test where id > $id::i64 and id < $id::i64 + 128000";
        let parser = StatefulVariableParser::new(query).unwrap();
        let var = parser.parse();
        assert!(
            matches!(var, Ok(Some(_))),
            "failed to parse query: {:?}",
            var
        );
        let (var, new_query) = var.unwrap().unwrap();
        assert_eq!(
            new_query,
            "SELECT * FROM test WHERE id > $1 AND id < $2 + 128000"
        );
        assert_eq!(
            var,
            StatefulVariable {
                name: "id".into(),
                value: StatefulVariableValue::I64(0),
                placeholder_count: 2
            }
        )
    }

    #[test]
    fn test_no_stateful_vars() {
        let query = "select * from test where id > 0 and id < 10";
        let parser = StatefulVariableParser::new(query).unwrap();
        let var = parser.parse();
        assert!(
            matches!(var, Ok(None)),
            "query should fail to parse: {:?}",
            var
        );
    }

    #[test]
    fn test_multiple_vars() {
        let query = "select * from test where id > $id::i64 and id < $id2::i64";
        let parser = StatefulVariableParser::new(query).unwrap();
        let var = parser.parse();
        assert!(var.is_err(), "query should fail to parse: {:?}", var);
    }
}
