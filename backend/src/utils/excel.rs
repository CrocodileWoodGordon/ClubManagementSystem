use std::{fs, io::Cursor, path::Path};

use calamine::{Data, Reader, Sheets, open_workbook_auto, open_workbook_auto_from_rs};
use uuid::Uuid;

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
    pub fn from_bytes(bytes: Vec<u8>, file_name: Option<&str>) -> Result<Self, AppError> {
        let cursor = Cursor::new(bytes.clone());
        match Self::from_reader(cursor) {
            Ok(workbook) => Ok(workbook),
            Err(primary_err) => match Self::from_temp_file(&bytes, file_name) {
                Ok(workbook) => Ok(workbook),
                Err(fallback_err) => Err(AppError::Parsing(format!(
                    "{}；回退方案失败: {}",
                    primary_err, fallback_err
                ))),
            },
        }
    }

    fn from_reader<R>(reader: R) -> Result<Self, AppError>
    where
        R: std::io::Read + std::io::Seek + Clone,
    {
        let workbook = open_workbook_auto_from_rs(reader)
            .map_err(|err| AppError::Parsing(format!("Excel 打开失败: {}", err)))?;
        Self::from_sheets(workbook)
    }

    fn from_temp_file(bytes: &[u8], file_name: Option<&str>) -> Result<Self, AppError> {
        let extension = infer_extension(bytes, file_name);
        let temp_path =
            std::env::temp_dir().join(format!("club_excel_{}.{}", Uuid::new_v4(), extension));
        fs::write(&temp_path, bytes)
            .map_err(|err| AppError::Parsing(format!("写入临时 Excel 文件失败: {}", err)))?;

        let result = open_workbook_auto(&temp_path)
            .map_err(|err| AppError::Parsing(format!("Excel 打开失败: {}", err)))
            .and_then(Self::from_sheets);

        let _ = fs::remove_file(&temp_path);
        result
    }

    fn from_sheets<RS>(mut workbook: Sheets<RS>) -> Result<Self, AppError>
    where
        RS: std::io::Read + std::io::Seek,
    {
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

    #[allow(dead_code)]
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

fn infer_extension(bytes: &[u8], file_name: Option<&str>) -> String {
    if let Some(name) = file_name {
        if let Some(ext) = Path::new(name).extension().and_then(|ext| ext.to_str()) {
            return ext.to_lowercase();
        }
    }

    if bytes.starts_with(b"\xD0\xCF\x11\xE0") {
        "xls".into()
    } else if bytes.starts_with(b"PK\x03\x04") {
        "xlsx".into()
    } else {
        "xlsx".into()
    }
}
