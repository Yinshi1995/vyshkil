//! Один плаский тип помилки для весь крейт — той самий підхід, що `DbErr::Custom(format!(...))`
//! у `app::backend::repo` (без `thiserror`, якого в цьому воркспейсі більше ніде нема).

use std::fmt;

#[derive(Debug)]
pub struct BusError(pub String);

impl fmt::Display for BusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for BusError {}

impl From<async_nats::ConnectError> for BusError {
    fn from(e: async_nats::ConnectError) -> Self {
        Self(format!("з'єднання з NATS: {e}"))
    }
}

impl From<serde_json::Error> for BusError {
    fn from(e: serde_json::Error) -> Self {
        Self(format!("серіалізація повідомлення: {e}"))
    }
}

pub type BusResult<T> = Result<T, BusError>;
