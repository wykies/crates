use futures::channel::oneshot;
use tracing::error;

#[derive(Debug)]
pub struct AwaitingType<T>(pub oneshot::Receiver<anyhow::Result<T>>);

#[derive(Debug, Default)]
pub enum DataState<T> {
    #[default]
    None,
    AwaitingResponse(AwaitingType<T>),
    Present(T),
    Failed(String),
}

impl<T> DataState<T> {
    /// Attempts to load the data and displays appropriate UI if applicable.
    /// Some branches lead to no UI being displayed, in particular when the data
    /// or an error is received (On the expectation it will show next frame).
    /// When in an error state the error messages will show as applicable.
    ///
    /// If a `retry_msg` is provided then it overrides the default
    ///
    /// Note see [`Self::get`] for more info.
    pub fn egui_get<F>(&mut self, ui: &mut egui::Ui, retry_msg: Option<&str>, fetch_fn: F)
    where
        F: FnOnce() -> AwaitingType<T>,
    {
        match self {
            DataState::None => {
                ui.spinner();
                self.get(fetch_fn);
            }
            DataState::AwaitingResponse(rx) => {
                if let Some(new_state) = Self::await_data(rx) {
                    *self = new_state;
                } else {
                    ui.spinner();
                }
            }
            DataState::Present(_data) => {
                // Does nothing as data is already present
            }
            DataState::Failed(e) => {
                ui.colored_label(ui.visuals().error_fg_color, format!("Request failed: {e}"));
                if ui.button(retry_msg.unwrap_or("Retry Request")).clicked() {
                    *self = DataState::default();
                }
            }
        }
    }

    /// Attempts to load the data.
    ///
    /// Note: F needs to return `AwaitingType<T>` and not T because it needs to
    /// be able to be pending if T is not ready.
    pub fn get<F>(&mut self, fetch_fn: F)
    where
        F: FnOnce() -> AwaitingType<T>,
    {
        match self {
            DataState::None => {
                let rx = fetch_fn();
                *self = DataState::AwaitingResponse(rx);
            }
            DataState::AwaitingResponse(rx) => {
                if let Some(new_state) = Self::await_data(rx) {
                    *self = new_state;
                }
            }
            DataState::Present(_data) => {
                // Does nothing data is already present
            }
            DataState::Failed(_e) => {
                // Have no way to let the user know there is an error nothing we
                // can do here
            }
        }
    }

    /// Checks to see if the data is ready and if it is returns a new [`Self`]
    /// otherwise None
    pub fn await_data(rx: &mut AwaitingType<T>) -> Option<Self> {
        Some(match rx.0.try_recv() {
            Ok(recv_opt) => match recv_opt {
                Some(outcome_result) => match outcome_result {
                    Ok(data) => DataState::Present(data),
                    Err(e) => {
                        let err_msg = format!("error: {e}");
                        error!(err_msg, "Error response received instead of the data");
                        DataState::Failed(err_msg)
                    }
                },
                None => {
                    return None;
                }
            },
            Err(e) => {
                let err_msg = format!("Error receiving on channel. Error: {e:?}");
                error!(err_msg, "Error receiving on channel");
                DataState::Failed(err_msg)
            }
        })
    }

    /// Returns `true` if the data state is [`Present`].
    ///
    /// [`Present`]: DataState::Present
    #[must_use]
    pub fn is_present(&self) -> bool {
        matches!(self, Self::Present(..))
    }

    /// Returns `true` if the data state is [`None`].
    ///
    /// [`None`]: DataState::None
    #[must_use]
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

impl<T> AsRef<DataState<T>> for DataState<T> {
    fn as_ref(&self) -> &DataState<T> {
        self
    }
}

impl<T> AsMut<DataState<T>> for DataState<T> {
    fn as_mut(&mut self) -> &mut DataState<T> {
        self
    }
}
