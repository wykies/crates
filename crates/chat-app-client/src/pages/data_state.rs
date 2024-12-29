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
    /// Attempts to load the data
    ///
    /// Some branches lead to no UI being displayed, in particular when the data
    /// is received or an error is received If a ui is passed then spinners
    /// and error messages will show as applicable
    ///
    /// Note: F needs to return AwaitingType<T> and not T because it needs to be
    /// able to be pending and T is not
    ///
    /// # PANIC
    /// Panics if the data is already present
    pub fn get<F>(&mut self, ui: Option<&mut egui::Ui>, retry_msg: Option<&str>, fetch_fn: F)
    where
        F: FnOnce() -> AwaitingType<T>,
    {
        match self {
            DataState::None => {
                if let Some(ui) = ui {
                    ui.spinner();
                }
                let rx = fetch_fn();
                *self = DataState::AwaitingResponse(rx);
            }
            DataState::AwaitingResponse(rx) => {
                if let Some(new_state) = Self::await_data(ui, rx) {
                    *self = new_state;
                }
            }
            DataState::Present(_data) => {
                // Panic because only reason I can think of that code got here is that there is
                // a bug in the calling code
                panic!("precondition not satisfied: Data is already present")
            }
            DataState::Failed(e) => {
                if let Some(ui) = ui {
                    ui.colored_label(ui.visuals().error_fg_color, format!("Request failed: {e}"));
                    if ui.button(retry_msg.unwrap_or("Retry Request")).clicked() {
                        *self = DataState::default();
                    }
                }
            }
        }
    }

    pub fn await_data(ui: Option<&mut egui::Ui>, rx: &mut AwaitingType<T>) -> Option<Self> {
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
                    if let Some(ui) = ui {
                        ui.spinner();
                    }
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
