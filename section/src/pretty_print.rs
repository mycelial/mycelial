//! pretty print dataframe

use crate::message::DataFrame;

struct PrettyPrint<'a> {
    df: &'a dyn DataFrame,
    limit: Option<usize>,
}

impl<'a> PrettyPrint<'a> {
    fn new(df: &'a dyn DataFrame, limit: Option<usize>) -> Self {
        Self { df, limit }
    }
}

impl std::fmt::Display for PrettyPrint<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let columns = self.df.columns();
        if columns.is_empty() {
            return Ok(());
        }
        let col_names = columns
            .iter()
            .map(|col| format!("{}::{}", col.name(), col.data_type()))
            .collect::<Vec<_>>();
        let values = columns
            .into_iter()
            .map(|col| col.map(|val| format!("{:?}", val)).collect::<Vec<String>>())
            .collect::<Vec<_>>();
        let max_lens = values
            .iter()
            .map(|val| val.iter().map(|s| s.len()).max().unwrap_or(0))
            .collect::<Vec<usize>>();
        let max_lens = max_lens
            .into_iter()
            .zip(col_names.iter())
            .map(|(ln, col_name)| col_name.len().max(ln))
            .collect::<Vec<usize>>();
        let max_lens = max_lens.as_slice();
        let header = format_header(max_lens, col_names.as_slice());
        let result = std::iter::once(header)
            .chain(format_values(max_lens, values.as_slice(), self.limit))
            .collect::<Vec<String>>()
            .join("\n");
        write!(f, "{}", result)
    }
}

fn format_frame(lens: &[usize]) -> String {
    lens.iter()
        .enumerate()
        .map(|(pos, len)| {
            if pos == 0 {
                format!("+{:->width$}+", "", width = len + 2)
            } else {
                format!("{:->width$}+", "", width = len + 2)
            }
        })
        .collect::<Vec<String>>()
        .join("")
}

fn format_row<T: AsRef<str>>(lens: &[usize], values: &[T]) -> String {
    lens.iter()
        .zip(values.iter())
        .enumerate()
        .map(|(pos, (len, name))| {
            if pos == 0 {
                format!("|{: >width$} |", name.as_ref(), width = len + 1)
            } else {
                format!("{: >width$} |", name.as_ref(), width = len + 1)
            }
        })
        .collect::<Vec<String>>()
        .join("")
}

fn format_header<T: AsRef<str>>(lens: &[usize], values: &[T]) -> String {
    let frame = format_frame(lens);
    let row = format_row(lens, values);
    [frame.as_str(), row.as_str(), frame.as_str()].join("\n")
}

fn format_values<T: AsRef<str>>(
    lens: &[usize],
    values: &[Vec<T>],
    limit: Option<usize>,
) -> Vec<String> {
    let frame = format_frame(lens);
    let frame = frame.as_str();
    let len = values[0].len();
    let len = match limit {
        Some(limit) if limit < len => limit,
        _ => len,
    };
    (0..len)
        .map(|row| {
            let values = values.iter().map(|val| &val[row]).collect::<Vec<_>>();
            let row = format_row(lens, values.as_slice());
            [row.as_str(), frame].join("\n")
        })
        .collect::<Vec<_>>()
}

pub fn pretty_print(df: &dyn DataFrame) -> String {
    PrettyPrint::new(df, None).to_string()
}

pub fn pretty_print_with_limit(df: &dyn DataFrame, len: usize) -> String {
    PrettyPrint::new(df, Some(len)).to_string()
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::message::{Column, DataFrame, DataType, ValueView};

    #[derive(Debug)]
    pub struct Values {
        ints: Vec<i64>,
        strings: Vec<String>,
    }

    impl DataFrame for Values {
        fn columns(&self) -> Vec<Column<'_>> {
            vec![
                Column::new(
                    "ints",
                    DataType::I64,
                    Box::new(self.ints.iter().map(|&x| ValueView::I64(x))),
                ),
                Column::new(
                    "strings",
                    DataType::Str,
                    Box::new(self.strings.iter().map(|s| ValueView::Str(s.as_str()))),
                ),
            ]
        }
    }

    #[test]
    fn test_format_header() {
        println!(
            "{}",
            format_header(&[1, 2], &["a".to_string(), "b".to_string()])
        );
    }

    #[test]
    fn test_pretty_print() {
        let v = Values {
            ints: vec![1, 2],
            strings: vec!["a".to_string(), "bbb".to_string()],
        };
        println!("{}", pretty_print(&v))
    }
}
