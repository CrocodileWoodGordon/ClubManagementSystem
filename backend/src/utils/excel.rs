use std::io::Cursor;

use calamine::{Data, Reader, Xlsx, open_workbook_from_rs};

use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct Worksheet {
    pub name: String,
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct ExcelWorkbook {
    pub sheets: Vec<Worksheet>,
}

impl ExcelWorkbook {
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, AppError> {
        let cursor = Cursor::new(bytes);
        let mut workbook: Xlsx<_> = open_workbook_from_rs(cursor)
            .map_err(|err| AppError::Parsing(format!("Excel 打开失败: {}", err)))?;
        let sheet_names = workbook.sheet_names().to_owned();
        let mut sheets = Vec::with_capacity(sheet_names.len());

        for name in sheet_names {
            if let Ok(range) = workbook.worksheet_range(&name) {
                let rows: Vec<Vec<String>> = range
                    .rows()
                    .map(|row| row.iter().map(cell_to_string).collect())
                    .collect();
                sheets.push(Worksheet { name, rows });
            }
        }

        if sheets.is_empty() {
            return Err(AppError::Parsing("Excel 文件缺少可读工作表".into()));
        }

        Ok(Self { sheets })
    }

    pub fn primary_sheet(&self) -> &Worksheet {
        &self.sheets[0]
    }

    pub fn sheet(&self, index: usize) -> Option<&Worksheet> {
        self.sheets.get(index)
    }
}

fn cell_to_string(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(value) => value.trim().to_string(),
        Data::Float(value) => {
            if (value.fract() - 0.0).abs() < f64::EPSILON {
                format!("{:.0}", value)
            } else {
                value.to_string()
            }
        }
        Data::Int(value) => value.to_string(),
        Data::Bool(value) => {
            if *value {
                "TRUE".into()
            } else {
                "FALSE".into()
            }
        }
        Data::DateTime(value) => value.to_string(),
        Data::DateTimeIso(value) => value.trim().to_string(),
        Data::DurationIso(value) => value.trim().to_string(),
        Data::Error(_) => String::new(),
    }
}
