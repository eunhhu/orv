use super::*;

pub(super) type ServerResponse = Response<RuntimeBody>;

pub(super) enum RuntimeBody {
    Full(Option<Bytes>),
    Trace(TraceEventBody),
}

impl RuntimeBody {
    pub(super) fn full(body: impl Into<Bytes>) -> Self {
        Self::Full(Some(body.into()))
    }

    pub(super) fn trace(initial: String, rx: tokio_mpsc::UnboundedReceiver<Bytes>) -> Self {
        Self::Trace(TraceEventBody {
            initial: Some(Bytes::from(initial)),
            rx,
        })
    }
}

impl HttpBody for RuntimeBody {
    type Data = Bytes;
    type Error = Infallible;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match &mut *self {
            Self::Full(body) => Poll::Ready(body.take().map(|bytes| Ok(Frame::data(bytes)))),
            Self::Trace(body) => Pin::new(body).poll_frame(cx),
        }
    }

    fn is_end_stream(&self) -> bool {
        matches!(self, Self::Full(None))
    }

    fn size_hint(&self) -> SizeHint {
        let mut hint = SizeHint::new();
        if let Self::Full(Some(bytes)) = self {
            hint.set_exact(bytes.len() as u64);
        }
        hint
    }
}

pub(super) struct TraceEventBody {
    initial: Option<Bytes>,
    rx: tokio_mpsc::UnboundedReceiver<Bytes>,
}

impl HttpBody for TraceEventBody {
    type Data = Bytes;
    type Error = Infallible;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        if let Some(initial) = self.initial.take() {
            return Poll::Ready(Some(Ok(Frame::data(initial))));
        }
        Pin::new(&mut self.rx)
            .poll_recv(cx)
            .map(|item| item.map(|bytes| Ok(Frame::data(bytes))))
    }

    fn is_end_stream(&self) -> bool {
        false
    }
}

pub(super) fn runtime_request_trace_path_from_env() -> Option<std::path::PathBuf> {
    std::env::var_os(ORV_RUNTIME_REQUEST_TRACE_PATH_ENV)
        .filter(|value| !value.is_empty())
        .map(std::path::PathBuf::from)
}
