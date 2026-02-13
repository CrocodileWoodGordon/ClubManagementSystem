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

pub fn encode_xlsx(sheet_name: &str, rows: &[Vec<String>]) -> Result<Vec<u8>, AppError> {
    let sheet_title = sanitize_sheet_name(sheet_name);
    let sheet_xml = build_sheet_xml(rows);
    let workbook_xml = build_workbook_xml(&sheet_title);
    let workbook_rels_xml = build_workbook_rels();
    let styles_xml = build_styles_xml();
    let content_types_xml = build_content_types();
    let package_rels_xml = build_package_rels();

    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let core_props_xml = build_core_props_xml(&sheet_title, &timestamp);
    let app_props_xml = build_app_props_xml();

    let files = vec![
        ("[Content_Types].xml", content_types_xml.into_bytes()),
        ("_rels/.rels", package_rels_xml.into_bytes()),
        ("xl/workbook.xml", workbook_xml.into_bytes()),
        ("xl/_rels/workbook.xml.rels", workbook_rels_xml.into_bytes()),
        ("xl/styles.xml", styles_xml.into_bytes()),
        ("xl/worksheets/sheet1.xml", sheet_xml.into_bytes()),
        ("docProps/core.xml", core_props_xml.into_bytes()),
        ("docProps/app.xml", app_props_xml.into_bytes()),
    ];

    Ok(build_zip_archive(files))
}

pub fn sanitize_file_name(name: &str) -> String {
    let mut sanitized: String = name
        .chars()
        .map(|ch| match ch {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => ch,
        })
        .collect();
    sanitized = sanitized.trim().to_string();
    if sanitized.is_empty() {
        sanitized = "attendance_template".into();
    }
    if !sanitized.to_ascii_lowercase().ends_with(".xlsx") {
        sanitized.push_str(".xlsx");
    }
    sanitized
}

fn sanitize_sheet_name(name: &str) -> String {
    let mut sanitized: String = name
        .chars()
        .map(|ch| match ch {
            '\\' | '/' | '?' | '*' | '[' | ']' | ':' => '_',
            _ => ch,
        })
        .collect();
    sanitized = sanitized.trim().to_string();
    if sanitized.is_empty() {
        sanitized = "Sheet1".into();
    }
    if sanitized.len() > 31 {
        sanitized.truncate(31);
    }
    sanitized
}

fn build_sheet_xml(rows: &[Vec<String>]) -> String {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>"#,
    );
    for (row_index, row) in rows.iter().enumerate() {
        let row_number = row_index + 1;
        xml.push_str(&format!("<row r=\"{}\">", row_number));
        for (col_index, cell) in row.iter().enumerate() {
            let cell_ref = format!("{}{}", column_reference(col_index + 1), row_number);
            if cell.is_empty() {
                xml.push_str(&format!("<c r=\"{}\"/>", cell_ref));
            } else {
                xml.push_str(&format!(
                    "<c r=\"{}\" t=\"inlineStr\"><is><t>{}</t></is></c>",
                    cell_ref,
                    xml_escape(cell)
                ));
            }
        }
        xml.push_str("</row>");
    }
    xml.push_str("</sheetData></worksheet>");
    xml
}

fn build_workbook_xml(sheet_name: &str) -> String {
    format!(
        concat!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
            r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" "#,
            r#"xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#,
            r#"<sheets><sheet name="{name}" sheetId="1" r:id="rId1"/></sheets></workbook>"#
        ),
        name = xml_escape(sheet_name)
    )
}

fn build_workbook_rels() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
</Relationships>"#.into()
}

fn build_styles_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <fonts count="1"><font><sz val="11"/><color theme="1"/><name val="Calibri"/><family val="2"/></font></fonts>
  <fills count="1"><fill><patternFill patternType="none"/></fill></fills>
  <borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>
  <cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>
  <cellXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/></cellXfs>
  <cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles>
</styleSheet>"#.into()
}

fn build_content_types() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
  <Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
  <Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>
  <Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>
  <Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml"/>
</Types>"#.into()
}

fn build_package_rels() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>"#.into()
}

fn build_core_props_xml(title: &str, timestamp: &str) -> String {
    format!(
        concat!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
            r#"<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" "#,
            r#"xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" "#,
            r#"xmlns:dcmitype="http://purl.org/dc/dcmitype/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">"#,
            r#"<dc:title>{title}</dc:title><dc:creator>Club Management System</dc:creator><cp:lastModifiedBy>Club Management System</cp:lastModifiedBy>"#,
            r#"<dcterms:created xsi:type="dcterms:W3CDTF">{timestamp}</dcterms:created>"#,
            r#"<dcterms:modified xsi:type="dcterms:W3CDTF">{timestamp}</dcterms:modified></cp:coreProperties>"#
        ),
        title = xml_escape(title),
        timestamp = timestamp
    )
}

fn build_app_props_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties" xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes">
  <Application>Club Management System</Application>
</Properties>"#.into()
}

fn column_reference(index: usize) -> String {
    let mut idx = index;
    let mut result = String::new();
    while idx > 0 {
        let rem = (idx - 1) % 26;
        result.insert(0, (b'A' + rem as u8) as char);
        idx = (idx - 1) / 26;
    }
    result
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace("'", "&apos;")
}

fn build_zip_archive(files: Vec<(&str, Vec<u8>)>) -> Vec<u8> {
    let mut archive = Vec::new();
    let mut central_directory = Vec::new();
    let mut offsets = Vec::with_capacity(files.len());

    for (name, data) in &files {
        let name_bytes = name.as_bytes();
        let crc = crc32(data);
        let size = data.len() as u32;
        let header_offset = archive.len() as u32;
        offsets.push(header_offset);

        write_u32_le(&mut archive, 0x0403_4B50);
        write_u16_le(&mut archive, 10); // version needed to extract
        write_u16_le(&mut archive, 0); // general purpose flag
        write_u16_le(&mut archive, 0); // compression method (store)
        write_u16_le(&mut archive, 0); // last mod time
        write_u16_le(&mut archive, 0); // last mod date
        write_u32_le(&mut archive, crc);
        write_u32_le(&mut archive, size);
        write_u32_le(&mut archive, size);
        write_u16_le(&mut archive, name_bytes.len() as u16);
        write_u16_le(&mut archive, 0); // extra field length
        archive.extend_from_slice(name_bytes);
        archive.extend_from_slice(data);
    }

    let central_dir_offset = archive.len() as u32;
    for ((name, data), offset) in files.iter().zip(offsets.iter()) {
        let name_bytes = name.as_bytes();
        let size = data.len() as u32;
        let crc = crc32(data);

        write_u32_le(&mut central_directory, 0x0201_4B50);
        write_u16_le(&mut central_directory, 20); // version made by
        write_u16_le(&mut central_directory, 10); // version needed to extract
        write_u16_le(&mut central_directory, 0); // general purpose
        write_u16_le(&mut central_directory, 0); // compression
        write_u16_le(&mut central_directory, 0); // mod time
        write_u16_le(&mut central_directory, 0); // mod date
        write_u32_le(&mut central_directory, crc);
        write_u32_le(&mut central_directory, size);
        write_u32_le(&mut central_directory, size);
        write_u16_le(&mut central_directory, name_bytes.len() as u16);
        write_u16_le(&mut central_directory, 0); // extra length
        write_u16_le(&mut central_directory, 0); // comment length
        write_u16_le(&mut central_directory, 0); // disk number start
        write_u16_le(&mut central_directory, 0); // internal attrs
        write_u32_le(&mut central_directory, 0); // external attrs
        write_u32_le(&mut central_directory, *offset);
        central_directory.extend_from_slice(name_bytes);
    }

    let central_dir_size = central_directory.len() as u32;
    archive.extend_from_slice(&central_directory);

    write_u32_le(&mut archive, 0x0605_4B50);
    write_u16_le(&mut archive, 0); // disk number
    write_u16_le(&mut archive, 0); // disk with central directory
    write_u16_le(&mut archive, files.len() as u16);
    write_u16_le(&mut archive, files.len() as u16);
    write_u32_le(&mut archive, central_dir_size);
    write_u32_le(&mut archive, central_dir_offset);
    write_u16_le(&mut archive, 0); // comment length

    archive
}

fn write_u16_le(target: &mut Vec<u8>, value: u16) {
    target.extend_from_slice(&value.to_le_bytes());
}

fn write_u32_le(target: &mut Vec<u8>, value: u32) {
    target.extend_from_slice(&value.to_le_bytes());
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 == 1 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}
