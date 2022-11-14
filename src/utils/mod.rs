use crate::Context;

use self::responses::Responses;

pub mod cli;
pub mod config;
pub mod message_context;
pub mod responses;
pub mod strings;

// Util
pub async fn validate_page(ctx: Context<'_>, page: Option<i64>) -> Result<usize, String> {
    let page = match page {
        Some(page) => page,
        None => 1,
    };

    if page <= 0 {
        ctx.warn("Page no must be atleast 1").await;
        return Err("Page no must be atleast 1".to_string());
    }

    Ok(page as usize - 1)
}

pub fn ceil(a: usize, b: usize) -> usize {
    (a + b - 1) / b
}
