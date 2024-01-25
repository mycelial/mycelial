//! Excel connector source for full-syncing excel files.
//!
//! # Details
//! - Uses `notify` crate to detect changes to excel files.
//! - Syncs an entire file when a change is detected.
//! - Uses glob pattern for matching filepaths / directories. (e.g. ** for recursive, * for all files)
//! - For sheets, use * to sync all sheets, otherwise a list of strings.

use crate::{ExcelDataTypeWrapper, ExcelMessage, ExcelPayload, Sheet, TableColumn};
use notify::{Event, RecursiveMode, Watcher};
use section::{
    command_channel::{Command, SectionChannel},
    futures::{self, FutureExt, Sink, SinkExt, Stream, StreamExt},
    message::{DataType, Value},
    section::Section,
    state::State,
    SectionError, SectionMessage,
};

// FIXME: drop direct dependency
use tokio::sync::mpsc::Sender;
use tokio_stream::wrappers::ReceiverStream;

use calamine::{open_workbook_auto, DataType as ExcelDataType, Reader, Rows};
use std::path::Path;
use std::pin::{pin, Pin};
use std::time::Duration;
use std::{future::Future, sync::Arc};

use glob::glob;
use globset::Glob;

#[derive(Debug)]
pub struct Excel {
    path: String,
    sheets: Vec<String>,
    stringify: bool,
}

#[derive(Debug)]
enum FsEvent {
    Change(String),
}

impl Excel {
    pub fn new(path: impl Into<String>, sheets: &[&str], stringify: bool) -> Self {
        Self {
            path: path.into(),
            sheets: sheets.iter().map(|&x| x.into()).collect(),
            stringify,
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

        // on init, sync all the files.
        for entry in glob(path).expect("Failed to read glob pattern") {
            match entry {
                Ok(path) => tx.send(FsEvent::Change(path.display().to_string())).await?,
                Err(_) => todo!(),
            }
        }

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
                                // When there's a change to a file, sync that file
                                FsEvent::Change(path) => {
                                    // ignore temp files
                                    if !path.contains('~') {
                                        let mut workbook: calamine::Sheets<std::io::BufReader<std::fs::File>> =
                                            open_workbook_auto(path.clone()).expect("Cannot open file");

                                        let mut sheets = self.init_schema(&mut workbook).await?;

                                        for sheet in sheets.iter_mut() {
                                            if let Some(Ok(range)) = workbook.worksheet_range(&sheet.name) {
                                                // the first is the header, so don't send it in the data payload since it's already part of the schema
                                                let mut rows = range.rows();
                                                rows.next();
                                                let excel_payload = self.build_excel_payload(sheet, rows)?;

                                                let origin: Arc<str> = Arc::from(format!("{}:{}", path, sheet.name));
                                                let message = Box::new(
                                                    ExcelMessage::new(
                                                        origin,
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
    ) -> Result<ExcelPayload, SectionError> {
        let mut values: Vec<Vec<Value>> = (0..sheet.columns.len())
            .map(|_| Vec::with_capacity(rows.len()))
            .collect();
        for row in rows {
            for (index, raw_value) in row.iter().enumerate() {
                values[index].push(ExcelDataTypeWrapper::new(raw_value, self.stringify).into());
            }
        }
        let batch = ExcelPayload {
            columns: Arc::clone(&sheet.columns),
            values,
        };
        Ok(batch)
    }

    async fn init_schema(
        &self,
        workbook: &mut calamine::Sheets<std::io::BufReader<std::fs::File>>,
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

        let data_type = match self.stringify {
            true => DataType::Str,
            false => DataType::Any,
        };

        for s in sheet_names {
            if let Some(Ok(range)) = workbook.worksheet_range(s) {
                let name = s.as_str();

                let mut rows = range.rows();
                let first_row = rows.next().ok_or("no rows")?;

                // get the column names from the first row
                let cols = first_row
                    .iter()
                    .map(|cell| {
                        let name = match cell {
                            ExcelDataType::String(s) => s.to_string(),
                            ExcelDataType::Int(i) => i.to_string(),
                            ExcelDataType::Float(f) => f.to_string(),
                            ExcelDataType::Bool(b) => b.to_string(),
                            ExcelDataType::DateTime(_) => cell
                                .as_datetime()
                                .unwrap()
                                .format("%Y-%m-%d %H:%M:%S")
                                .to_string(),
                            ExcelDataType::Duration(f) => f.to_string(),
                            ExcelDataType::DateTimeIso(_) => cell
                                .as_datetime()
                                .unwrap()
                                .format("%Y-%m-%d %H:%M:%S")
                                .to_string(),
                            ExcelDataType::DurationIso(f) => f.to_string(),
                            ExcelDataType::Error(e) => e.to_string(),
                            ExcelDataType::Empty => "".to_string(),
                        };

                        TableColumn {
                            name: Arc::from(name),
                            data_type,
                        }
                    })
                    .collect::<Vec<TableColumn>>();

                let sheet = Sheet {
                    name: Arc::from(name),
                    columns: Arc::from(cols),
                };
                sheets.push(sheet);
            }
        }
        Ok(sheets)
    }

    fn watch_excel_paths(
        &self,
        excel_path: &str,
        tx: Sender<FsEvent>,
    ) -> notify::Result<impl Watcher> {
        let pattern = Glob::new(excel_path).unwrap().compile_matcher();
        let mut file_watcher = notify::PollWatcher::new(
            move |res: Result<Event, _>| match res {
                Ok(event) if event.kind.is_modify() || event.kind.is_create() => {
                    let path: String = event.paths[0].to_str().unwrap().into();
                    if pattern.is_match(path.as_str()) {
                        tx.blocking_send(FsEvent::Change(path)).ok();
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

// gets the root directory of the path to watch, or the path itself if there are no wildcards
fn get_directory_or_filepath(path: &str) -> String {
    let path = path.to_string();
    let split_path = path.split('/').collect::<Vec<&str>>();
    let mut directory_to_watch = String::new();
    let mut found = false;
    for part in split_path.iter() {
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
