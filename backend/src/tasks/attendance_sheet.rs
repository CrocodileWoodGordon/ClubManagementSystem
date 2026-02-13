#![allow(dead_code)]

use crate::domain::ClassInstance;
use crate::error::AppError;

/// Long running task that renders printable attendance sheets for a class.
pub async fn generate_for_class(_class: &ClassInstance) -> Result<Vec<u8>, AppError> {
    Ok(Vec::new())
}
