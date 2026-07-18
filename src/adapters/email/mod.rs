pub mod composite;
pub mod resend;
pub mod smtp;

pub use composite::{build_mailer, CompositeMailer};
pub use resend::ResendMailer;
pub use smtp::{smtp_with_creds, NullMailer, SmtpMailer};
