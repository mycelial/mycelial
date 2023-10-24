use crate::{ColumnType, ExcelPayload, Message, StdError, Value};
use futures::{FutureExt, Sink, SinkExt, Stream, StreamExt};
use notify::{Event, RecursiveMode, Watcher};
use section::{Command, Section, SectionChannel, State};

// FIXME: drop direct dependency
use tokio::sync::mpsc::Sender;
use tokio_stream::wrappers::ReceiverStream;

use calamine::{open_workbook_auto, DataType, Reader, Rows};
use std::path::Path;
use std::pin::{pin, Pin};
use std::{future::Future, sync::Arc};

fn err_msg(msg: impl Into<String>) -> StdError {
    msg.into().into()
}

#[derive(Debug)]
pub struct Excel {
    path: String,
    sheets: Vec<String>,
}

impl TryFrom<(ColumnType, usize, &[DataType])> for Value {
    // FIXME: specific error instead of Box<dyn Error>
    type Error = StdError;

    fn try_from((col, index, row): (ColumnType, usize, &[DataType])) -> Result<Self, Self::Error> {
        let value = match col {
            ColumnType::Int => row
                .get(index)
                .ok_or(err_msg("oh no"))
                .unwrap()
                .get_int()
                .map(Value::Int), //FIXME: unwrap
            ColumnType::Text => row
                .get(index)
                .ok_or(err_msg("oh no"))
                .unwrap()
                .get_string()
                .map(|s| Value::Text(s.into())), //FIXME: unwrap
            ColumnType::Real => row
                .get(index)
                .ok_or(err_msg("oh no"))
                .unwrap()
                .get_float()
                .map(|f| Value::Real(f.into())),
            ColumnType::Bool => row
                .get(index)
                .ok_or(err_msg("oh no"))
                .unwrap()
                .get_bool()
                .map(|b| Value::Bool(b.into())), //FIXME: unwrap
            ColumnType::DateTime => row
                .get(index)
                .ok_or(err_msg("oh no"))
                .unwrap()
                .as_datetime()
                .map(|b| Value::DateTime(b.into())), //FIXME: unwrap;
            _ => {
                return Err(format!(
                    "TryFrom<(ColumnType, usize, &[DataType])> for Value - unimplemented: {:?}",
                    col
                )
                .into())
            }
        };
        let v = value.unwrap_or(Value::Null); // FIXME? Here's where we're ignoring errors
        Ok(v)
    }
}

#[derive(Debug)]
pub struct Sheet {
    pub name: Arc<str>,
    pub columns: Arc<[String]>,
    pub column_types: Arc<[ColumnType]>,
}

#[derive(Debug)]
enum InnerEvent {
    NewChange,
}

impl Excel {
    pub fn new(path: impl Into<String>, sheets: &[&str]) -> Self {
        Self {
            path: path.into(),
            sheets: sheets.iter().map(|&x| x.into()).collect(),
        }
    }

    async fn enter_loop<Input, Output, SectionChan>(
        self,
        input: Input,
        output: Output,
        mut section_channel: SectionChan,
    ) -> Result<(), StdError>
    where
        Input: Stream + Send,
        Output: Sink<Message, Error = StdError> + Send,
        SectionChan: SectionChannel + Send + Sync,
    {
        // TOOD: get this from config//self
        let path = &self.path;
        let mut workbook: calamine::Sheets<std::io::BufReader<std::fs::File>> =
            open_workbook_auto(path).expect("Cannot open file");

        let (tx, rx) = tokio::sync::mpsc::channel(4);
        tx.send(InnerEvent::NewChange).await?;

        let _watcher = self.watch_excel_path(self.path.as_str(), tx);

        let mut _input = pin!(input.fuse());
        let mut output = pin!(output);

        let mut state = section_channel
            .retrieve_state()
            .await?
            .unwrap_or(<<SectionChan as SectionChannel>::State>::new());

        let mut sheets = self
            .init_schema::<SectionChan>(&mut workbook, &state)
            .await?;

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
                        Some(_) => {},
                        None => Err("excel file watched exited")?
                    };
                    for sheet in sheets.iter_mut() {
                        if let Some(Ok(range)) = workbook.worksheet_range(&sheet.name) {
                            let mut rows = range.rows();
                            // the first is the header, so don't send it in the data payload since it's already part of the schema
                            let _ = rows.next();
                            let excel_payload = self.build_excel_payload(sheet, rows)?;

                            let message = Message::new(
                                sheet.name.to_string(),
                                excel_payload,
                                None,
                            );
                            output.send(message).await.map_err(|e| format!("failed to send data to sink {:?}", e))?;
                        }

                    }
                }
            }
        }
    }

    fn build_excel_payload(
        &self,
        sheet: &Sheet,
        rows: Rows<calamine::DataType>,
    ) -> Result<ExcelPayload, StdError> {
        let mut values: Vec<Vec<Value>> = vec![];
        let cap = rows.len();
        for row in rows {
            if values.len() != row.len() {
                values = row.iter().map(|_| Vec::with_capacity(cap)).collect();
            }
            for (index, column) in sheet.column_types.iter().enumerate() {
                let value = Value::try_from((*column, index, row))?;
                values[index].push(value);
            }
        }
        let batch = ExcelPayload {
            columns: Arc::clone(&sheet.columns),
            column_types: Arc::clone(&sheet.column_types),
            values,
            offset: 0,
        };
        Ok(batch)
    }

    async fn init_schema<C: SectionChannel>(
        &self,
        workbook: &mut calamine::Sheets<std::io::BufReader<std::fs::File>>,
        _state: &<C as SectionChannel>::State,
    ) -> Result<Vec<Sheet>, StdError> {
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
            if let Some(Ok(range)) = workbook.worksheet_range(&s) {
                let name = s.as_str();

                let mut rows = range.rows();
                let first_row = rows.next().ok_or(err_msg("no rows"))?;
                let second_row = rows.next().ok_or(err_msg("no rows"))?;

                // the column names are in the first row, which we want to put into `cols`
                let cols = first_row
                    .iter()
                    .map(|cell| match cell {
                        DataType::String(s) => Ok(s.to_string()),
                        _ => Err(err_msg("column name is not a string")),
                    })
                    .collect::<Result<Vec<String>, StdError>>()?;
                // get the data types from teh second row
                let col_types: Vec<ColumnType> = second_row
                    .iter()
                    .map(|cell| match cell {
                        DataType::String(_) => ColumnType::Text,
                        DataType::Int(_) => ColumnType::Int,
                        DataType::Float(_) => ColumnType::Real,
                        DataType::Bool(_) => ColumnType::Bool,
                        DataType::DateTime(_) => ColumnType::DateTime,
                        DataType::Duration(_) => ColumnType::Duration,
                        DataType::DateTimeIso(_) => ColumnType::DateTimeIso,
                        DataType::DurationIso(_) => ColumnType::DurationIso,
                        _ => ColumnType::Text,
                    })
                    .collect();

                let sheet = Sheet {
                    name: Arc::from(name),
                    columns: Arc::from(cols),
                    column_types: Arc::from(col_types),
                };
                sheets.push(sheet);
            }
        }

        Ok(sheets)
    }

    fn watch_excel_path(
        &self,
        excel_path: &str,
        tx: Sender<InnerEvent>,
    ) -> notify::Result<impl Watcher> {
        // initiate first check on startup
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| match res {
            Ok(event) if event.kind.is_modify() || event.kind.is_create() => {
                tx.blocking_send(InnerEvent::NewChange).ok();
            }
            Ok(_) => (),
            Err(_e) => (),
        })?;
        // watch excel file
        let _ = &[Path::new(excel_path)]
            .into_iter()
            .filter(|path| path.exists())
            .try_for_each(|path| watcher.watch(path, RecursiveMode::NonRecursive))?;
        Ok(watcher)
    }
}

struct AckMessage {
    sheet: Arc<str>,
    offset: i64,
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for Excel
where
    Input: Stream + Send + 'static,
    Output: Sink<Message, Error = StdError> + Send + 'static,
    SectionChan: SectionChannel + Send + Sync + 'static,
{
    type Error = StdError;
    type Future = Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'static>>;

    fn start(self, input: Input, output: Output, command: SectionChan) -> Self::Future {
        Box::pin(async move { self.enter_loop(input, output, command).await })
    }
}

pub fn new(path: impl Into<String>, sheets: &[&str]) -> Excel {
    Excel::new(path, sheets)
}
