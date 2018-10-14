use std::result;

pub struct Error(Box<ErrorImpl>);

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum ErrorImpl {}
