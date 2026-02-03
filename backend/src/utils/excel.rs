use calamine::{Reader, Xlsx, open_workbook_from_rs};

use crate::error::AppError;

#[derive(Debug)]
pub struct ExcelWorkbook {
    pub sheet_names: Vec<String>,
}

impl ExcelWorkbook {
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, AppError> {
        let mut workbook: Xlsx<_> = open_workbook_from_rs(std::io::Cursor::new(bytes))
            .map_err(|err| AppError::Parsing(err.to_string()))?;
        let sheet_names = workbook
            .sheet_names()
            .iter()
            .map(|name| name.to_string())
            .collect();
        Ok(Self { sheet_names })
    }
}
