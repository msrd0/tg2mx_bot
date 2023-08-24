use std::fmt::{self, Display, Formatter, Write as _};

pub(super) enum ErrorSource<'a> {
	Std(&'a dyn std::error::Error),
	Anyhow(&'a anyhow::Error)
}

impl Display for ErrorSource<'_> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		match self {
			Self::Std(err) => err.fmt(f),
			Self::Anyhow(err) => err.fmt(f)
		}
	}
}

impl<'a> ErrorSource<'a> {
	fn source(&'a self) -> Option<ErrorSource<'a>> {
		match self {
			Self::Std(err) => err.source(),
			Self::Anyhow(err) => err.source()
		}
		.map(Self::Std)
	}
}

impl<'a> From<&'a anyhow::Error> for ErrorSource<'a> {
	fn from(err: &'a anyhow::Error) -> Self {
		Self::Anyhow(err)
	}
}

pub(super) fn build_err_msg<'e, E: Into<ErrorSource<'e>>>(err: E) -> String {
	let mut msg = String::new();
	build_err_msg_impl(&mut msg, err.into());
	msg
}

fn build_err_msg_impl(msg: &mut String, err: ErrorSource<'_>) {
	writeln!(msg, "{err}").unwrap();
	let has_list = err.source().is_some();
	if has_list {
		writeln!(msg, "<ul>").unwrap();
	}
	if let Some(source) = err.source() {
		writeln!(msg, "<li><b>Caused by:</b>").unwrap();
		build_err_msg_impl(msg, source);
		writeln!(msg, "</li>").unwrap();
	}
	if has_list {
		writeln!(msg, "</ul>").unwrap();
	}
}
