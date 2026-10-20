//! Narrow CLI-only facts retained inside io::Error, never recovered from prose.

use std::fmt;
use std::io;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CompletionChannel {
    Output,
    Terminal,
}

#[derive(Debug)]
pub(crate) enum CliIoError {
    CurrentDirectory {
        source: io::Error,
    },
    DirectoryName {
        directory: PathBuf,
        source: Option<hydragen::InitError>,
    },
    Input {
        source: io::Error,
    },
    CancellationFeedback {
        source: io::Error,
    },
    InitCompletion {
        path: PathBuf,
        channel: CompletionChannel,
        source: io::Error,
    },
}

impl CliIoError {
    pub fn into_io(self) -> io::Error {
        let kind = match &self {
            Self::DirectoryName { .. } => io::ErrorKind::InvalidInput,
            Self::CurrentDirectory { source }
            | Self::Input { source }
            | Self::CancellationFeedback { source }
            | Self::InitCompletion { source, .. } => source.kind(),
        };
        io::Error::new(kind, self)
    }
}

impl fmt::Display for CliIoError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&super::operations::io_context(self).body())
    }
}

impl std::error::Error for CliIoError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::DirectoryName { source, .. } => source.as_ref().map(|source| source as _),
            Self::CurrentDirectory { source }
            | Self::Input { source }
            | Self::CancellationFeedback { source }
            | Self::InitCompletion { source, .. } => Some(source),
        }
    }
}
