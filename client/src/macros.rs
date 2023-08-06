// FIXME: move this macro to some crate or smth
#[macro_export]
macro_rules! call {
    ($self:ident, $ty:tt :: $arm:tt { $($field:tt: $value:expr),* $(,)?} ) => {
        {
            let (reply_to, rx) = oneshot_channel();
            $self.send($ty::$arm{
                $($field: $value,)*
                reply_to,
            }).await?;
            rx.await?
        }
    };
    // shortcut struct init
    ($self:ident, $ty:tt :: $arm:tt { $($field:tt),* $(,)?} ) => {
        {
            let (reply_to, rx) = oneshot_channel();
            $self.send($ty::$arm{
                $($field,)*
                reply_to,
            }).await?;
            rx.await?
        }
    }
}

