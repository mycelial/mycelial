use crate::{ColumnType, ExcelPayload, Sheet, TableColumn, NewExcelPayload, ExcelMessage};
use notify::{Event, RecursiveMode, Watcher};
use section::{
    command_channel::{Command, SectionChannel, WeakSectionChannel},
    futures::{self, FutureExt, Sink, SinkExt, Stream, StreamExt},
    message::{DataType, Value},
    section::Section,
    state::State,
    SectionError, SectionFuture, SectionMessage,
};

// FIXME: drop direct dependency
use tokio::sync::mpsc::Sender;
use tokio_stream::wrappers::ReceiverStream;

use calamine::{open_workbook_auto, DataType as ExcelDataType, Reader, Rows, Table};
use std::path::Path;
use std::pin::{pin, Pin};
use std::time::Duration;
use std::{future::Future, sync::Arc};

use glob::glob;
use globset::Glob;

fn err_msg(msg: impl Into<String>) -> SectionError {
    msg.into().into()
}

#[derive(Debug)]
pub struct Excel {
    path: String,
    sheets: Vec<String>,
    strict: bool,
}

// impl TryFrom<(ColumnType, usize, &[ExcelDataType], bool)> for Value {
//     // FIXME: specific error instead of Box<dyn Error>
//     type Error = SectionError;

//     fn try_from(
//         (col, index, row, strict): (ColumnType, usize, &[ExcelDataType], bool),
//     ) -> Result<Self, Self::Error> {
//         if !strict {
//             let v2 = row.get(index).unwrap();
//             let v = match v2 {
//                 ExcelDataType::Int(v) => v.to_string(),
//                 ExcelDataType::Float(f) => f.to_string(),
//                 ExcelDataType::String(s) => s.to_string(),
//                 ExcelDataType::Bool(b) => b.to_string(),
//                 ExcelDataType::DateTime(_) => v2
//                     .as_datetime()
//                     .unwrap()
//                     .format("%Y-%m-%d %H:%M:%S")
//                     .to_string(),
//                 ExcelDataType::Duration(d) => d.to_string(),
//                 ExcelDataType::DateTimeIso(d) => d.to_string(),
//                 ExcelDataType::DurationIso(d) => d.to_string(),
//                 ExcelDataType::Error(e) => e.to_string(),
//                 ExcelDataType::Empty => "".to_string(),
//             };
//             let v = Value::Text(v);
//             return Ok(v);
//         }

//         let value = match col {
//             ColumnType::Int => row
//                 .get(index)
//                 .ok_or(err_msg("oh no"))
//                 .unwrap()
//                 .get_int()
//                 .map(Value::Int), //FIXME: unwrap
//             ColumnType::Text => row
//                 .get(index)
//                 .ok_or(err_msg("oh no"))
//                 .unwrap()
//                 .get_string()
//                 .map(|s| Value::Text(s.into())), //FIXME: unwrap
//             ColumnType::Real => row
//                 .get(index)
//                 .ok_or(err_msg("oh no"))
//                 .unwrap()
//                 .get_float()
//                 .map(Value::Real),
//             ColumnType::Bool => row
//                 .get(index)
//                 .ok_or(err_msg("oh no"))
//                 .unwrap()
//                 .get_bool()
//                 .map(Value::Bool), //FIXME: unwrap
//             ColumnType::DateTime => row
//                 .get(index)
//                 .ok_or(err_msg("oh no"))
//                 .unwrap()
//                 .as_datetime()
//                 .map(Value::DateTime), //FIXME: unwrap;
//             _ => {
//                 return Err(format!(
//                     "TryFrom<(ColumnType, usize, &[ExcelDataType])> for Value - unimplemented: {:?}",
//                     col
//                 )
//                 .into())
//             }
//         };
//         let v = value.unwrap_or(Value::Null); // FIXME? Here's where we're ignoring errors
//         Ok(v)
//     }
// }

// #[derive(Debug)]
// pub struct Sheet {
//     pub name: Arc<str>,
//     pub columns: Arc<[String]>,
//     pub column_types: Arc<[ColumnType]>,
// }

#[derive(Debug)]
enum InnerEvent {
    Init,
    NewChange(String),
}

impl Excel {
    pub fn new(path: impl Into<String>, sheets: &[&str], strict: bool) -> Self {
        Self {
            path: path.into(),
            sheets: sheets.iter().map(|&x| x.into()).collect(),
            strict,
        }
    }

    async fn enter_loop<Input, Output, SectionChan>(
        self,
        input: Input,
        output: Output,
        mut section_channel: SectionChan,
    ) -> Result<(), SectionError>
    where
        Input: Stream + Send,
        Output: Sink<SectionMessage, Error = SectionError> + Send,
        SectionChan: SectionChannel + Send + Sync,
    {
        let path = &self.path;
        let (tx, rx) = tokio::sync::mpsc::channel(4);
        tx.send(InnerEvent::Init).await?;

        let _watcher = self.watch_excel_paths(self.path.as_str(), tx);

        let mut _input = pin!(input.fuse());
        let mut output = pin!(output);

        let mut state = section_channel
            .retrieve_state()
            .await?
            .unwrap_or(<<SectionChan as SectionChannel>::State>::new());
        let rx = ReceiverStream::new(rx);
        let mut rx = pin!(rx.fuse());

        loop {
            futures::select_biased! {
                cmd = section_channel.recv().fuse() => {
                    match cmd? {
                        Command::Ack(any) => {
                            match any.downcast::<AckMessage>() {
                                Ok(ack) => {
                                    state.set(&ack.sheet, ack.offset)?;
                                    section_channel.store_state(state.clone()).await?;
                                },
                                Err(_) =>
                                    Err("Failed to downcast incoming Ack message to Message")?,
                            };
                        },
                        Command::Stop => return Ok(()),
                        _ => {},
                    }
                },
                msg = rx.next() => {
                    match msg {
                        Some(event) => {
                            match event {
                                // on init, sync all the files.
                                InnerEvent::Init => {
                                    for entry in glob(path).expect("Failed to read glob pattern") {
                                        match entry {
                                            Ok(path) => {
                                                let p = path.display().to_string();
                                                // ignore temp files
                                                if !p.contains('~') {
                                                    let mut workbook: calamine::Sheets<std::io::BufReader<std::fs::File>> =
                                                        open_workbook_auto(path).expect("Cannot open file");

                                                    let mut sheets = self
                                                        .init_schema::<SectionChan>(&mut workbook, &state, self.strict)
                                                        .await?;

                                                    for sheet in sheets.iter_mut() {
                                                        if let Some(Ok(range)) = workbook.worksheet_range(&sheet.name) {
                                                            let mut rows = range.rows();
                                                            // the first is the header, so don't send it in the data payload since it's already part of the schema
                                                            let _ = rows.next();
                                                            let excel_payload = self.build_excel_payload(sheet, rows, self.strict)?;

                                                            let message = Box::new(ExcelMessage::new(
                                                                // format!("{}:{}", p, sheet.name),
                                                                Arc::clone(&sheet.name), // todo: add filepath
                                                                excel_payload,
                                                                None,
                                                            ));
                                                            output.send(message).await.map_err(|e| format!("failed to send data to sink {:?}", e))?;
                                                        }

                                                    }
                                                }
                                            },
                                            Err(e) => println!("{:?}", e),
                                        }
                                    }
                                }
                                // When there's a change to a file, sync that file
                                InnerEvent::NewChange(path) => {
                                    // ignore temp files
                                    if !path.contains('~') {
                                        let mut workbook: calamine::Sheets<std::io::BufReader<std::fs::File>> =
                                            open_workbook_auto(path).expect("Cannot open file");

                                        let mut sheets = self
                                            .init_schema::<SectionChan>(&mut workbook, &state, self.strict)
                                            .await?;

                                        for sheet in sheets.iter_mut() {
                                            if let Some(Ok(range)) = workbook.worksheet_range(&sheet.name) {
                                                let mut rows = range.rows();
                                                // the first is the header, so don't send it in the data payload since it's already part of the schema
                                                let _ = rows.next();
                                                let excel_payload = self.build_excel_payload(sheet, rows, self.strict)?;

                                                // let message = Message::new(
                                                //     sheet.name.to_string(),
                                                //     excel_payload,
                                                //     None,
                                                // );

                                                // let origin = format!("{}:{}", path, &sheet.name);
                                                let message = Box::new(
                                                    ExcelMessage::new(
                                                        Arc::clone(&sheet.name), // todo: add filepath
                                                        excel_payload,
                                                        None,
                                                    )
                                                );
                                                output.send(message).await.map_err(|e| format!("failed to send data to sink {:?}", e))?;
                                            }

                                        }
                                    }
                                },
                            }
                        },
                        None => Err("excel file watched exited")?
                    };

                }
            }
        }
    }

    fn build_excel_payload(
        &self,
        sheet: &Sheet,
        rows: Rows<ExcelDataType>,
        strict: bool,
    ) -> Result<NewExcelPayload, SectionError> {
        let mut values: Vec<Vec<Value>> = vec![];
        let cap = rows.len();
        for row in rows {
            if values.len() != row.len() {
                values = row.iter().map(|_| Vec::with_capacity(cap)).collect();
            }
            for (index, column) in sheet.columns.iter().enumerate() {
                let x = row.get(index).ok_or(err_msg("oh no"))?; //FIXME: unwrap
                let y = match x {
                    // TODO: Not all of these should be Str -- map to the correct Value type
                    ExcelDataType::Int(v) => Value::Str(v.to_string()),
                    ExcelDataType::Float(f) => Value::Str(f.to_string()),
                    ExcelDataType::String(s) => Value::Str(s.to_string()),
                    ExcelDataType::Bool(b) => Value::Str(b.to_string()),
                    ExcelDataType::DateTime(_) => Value::Str(x
                        .as_datetime()
                        .unwrap()
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string()),
                    ExcelDataType::Duration(d) => Value::Str(d.to_string()),
                    ExcelDataType::DateTimeIso(d) => Value::Str(d.to_string()),
                    ExcelDataType::DurationIso(d) => Value::Str(d.to_string()),
                    ExcelDataType::Error(e) => Value::Str(e.to_string()),
                    ExcelDataType::Empty => Value::Str("".to_string()),
                };
                values[index].push(y);
            }
        }
        let batch = NewExcelPayload {
            columns: Arc::clone(&sheet.columns),
            values,
        };
        Ok(batch)
    }

    async fn init_schema<C: SectionChannel>(
        &self,
        workbook: &mut calamine::Sheets<std::io::BufReader<std::fs::File>>,
        _state: &<C as SectionChannel>::State,
        strict: bool,
    ) -> Result<Vec<Sheet>, SectionError> {
        let mut sheets = Vec::with_capacity(self.sheets.len());
        let all_sheets: Vec<String>;
        let sheet_names = match self.sheets.iter().any(|sheet| sheet == "*") {
            true => {
                all_sheets = workbook.sheet_names().to_owned();
                all_sheets.as_slice()
            }
            false => self.sheets.as_slice(),
        };

        for s in sheet_names {
            if let Some(Ok(range)) = workbook.worksheet_range(s) {
                let name = s.as_str();

                let mut rows = range.rows();
                let first_row = rows.next().ok_or(err_msg("no rows"))?;
                let second_row = rows.next().ok_or(err_msg("no rows"))?;

                // the column names are in the first row, which we want to put into `cols`
                let cols = first_row
                    .iter()
                    .map(|cell| {
                        let name = match cell {
                            ExcelDataType::String(s) => Ok(s.to_string()),
                            _ => Err(err_msg("column name is not a string")),
                        }.unwrap();
                        TableColumn {
                            name: Arc::from(name.to_owned()),
                            data_type: DataType::Str,
                            nullable: true,
                        }
                    })
                    .collect::<Vec<TableColumn>>();
                    // .collect::()?;
                // get the data types from the second row
                // let col_types: Vec<ColumnType> = second_row
                //     .iter()
                //     .map(|cell| match strict {
                //         true => match cell {
                //             ExcelDataType::String(_) => ColumnType::Text,
                //             ExcelDataType::Int(_) => ColumnType::Int,
                //             ExcelDataType::Float(_) => ColumnType::Real,
                //             ExcelDataType::Bool(_) => ColumnType::Bool,
                //             ExcelDataType::DateTime(_) => ColumnType::DateTime,
                //             ExcelDataType::Duration(_) => ColumnType::Duration,
                //             ExcelDataType::DateTimeIso(_) => ColumnType::DateTimeIso,
                //             ExcelDataType::DurationIso(_) => ColumnType::DurationIso,
                //             _ => ColumnType::Text,
                //         },
                //         false => ColumnType::Text,
                //     })
                //     .collect();

                let sheet = Sheet {
                    name: Arc::from(name),
                    columns: Arc::from(cols),
                    // column_types: Arc::from(col_types),
                };
                sheets.push(sheet);
            }
        }
        Ok(sheets)
    }

    fn watch_excel_paths(
        &self,
        excel_path: &str,
        tx: Sender<InnerEvent>,
    ) -> notify::Result<impl Watcher> {
        let pattern = Glob::new(excel_path).unwrap().compile_matcher();
        let mut file_watcher = notify::PollWatcher::new(
            move |res: Result<Event, _>| match res {
                Ok(event) if event.kind.is_modify() || event.kind.is_create() => {
                    let path: String = event.paths[0].to_str().unwrap().into();
                    if pattern.is_match(path.as_str()) {
                        tx.blocking_send(InnerEvent::NewChange(path)).ok();
                    }
                }
                Ok(_) => (),
                Err(_) => (),
            },
            notify::Config::default().with_poll_interval(Duration::from_secs(1)),
        )?;
        let path_to_watch = get_directory_or_filepath(excel_path);
        let _ = file_watcher.watch(Path::new(&path_to_watch), RecursiveMode::Recursive);
        Ok(file_watcher)
    }
}

struct AckMessage {
    sheet: Arc<str>,
    offset: i64,
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for Excel
where
    Input: Stream + Send + 'static,
    Output: Sink<SectionMessage, Error = SectionError> + Send + 'static,
    SectionChan: SectionChannel + Send + Sync + 'static,
{
    type Error = SectionError;
    type Future = Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'static>>;

    fn start(self, input: Input, output: Output, command: SectionChan) -> Self::Future {
        Box::pin(async move { self.enter_loop(input, output, command).await })
    }
}

pub fn new(path: impl Into<String>, sheets: &[&str], strict: bool) -> Excel {
    Excel::new(path, sheets, strict)
}

// gets the root directory of the path to watch, or the path itself if there are no wildcards
fn get_directory_or_filepath(path: &str) -> String {
    let path = path.to_string();
    let split_path = path.split('/').collect::<Vec<&str>>();
    let mut directory_to_watch = String::new();
    let mut found = false;
    for (_i, part) in split_path.iter().enumerate() {
        if part.contains('*') || part.contains("**") {
            found = true;
            break;
        }
        directory_to_watch.push_str(part);
        directory_to_watch.push('/');
    }
    if found {
        directory_to_watch
    } else {
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_directory_or_filepath_with_single_star() {
        let path = "path/to/*/file.txt";
        let result = get_directory_or_filepath(path);
        assert_eq!(result, "path/to/");
    }

    #[test]
    fn test_get_directory_or_filepath_with_double_star() {
        let path = "path/to/**/file.txt";
        let result = get_directory_or_filepath(path);
        assert_eq!(result, "path/to/");
    }

    #[test]
    fn test_get_directory_or_filepath_with_star_in_filename() {
        let path = "path/to/file_*.txt";
        let result = get_directory_or_filepath(path);
        assert_eq!(result, "path/to/");
    }

    #[test]
    fn test_get_directory_or_filepath_without_star() {
        let path = "path/to/file.txt";
        let result = get_directory_or_filepath(path);
        assert_eq!(result, "path/to/file.txt");
    }
}
