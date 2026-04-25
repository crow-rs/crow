/// Panics with an error
#[macro_export]
macro_rules! bail {
    ($report:expr) => {{
        let report: miette::Report = $report.into();
        panic!("{report:?}");
    }};
}

/// Emits an error
#[macro_export]
macro_rules! emit {
    ($self:expr, $report:expr) => {{
        $self.has_error = true;
        let report: miette::Report = $report.into();
        println!("{report:?}");
    }};
}

/// Panics with bug error
#[macro_export]
macro_rules! bug {
    ($text:expr) => {{
        panic!("{:?}", miette::miette!($text));
    }};
}
