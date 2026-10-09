//! Outgoing email. Messages are queued as `send_email` jobs inside the
//! transaction that needs them, then delivered by the job worker.

use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor, message::header::ContentType,
};
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;

use crate::{Config, jobs};

pub const JOB_KIND: &str = "send_email";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Email {
    pub to: String,
    pub subject: String,
    pub body: String,
}

/// Queue `email` for delivery once the surrounding transaction commits.
pub async fn queue(conn: &mut PgConnection, email: &Email) -> sqlx::Result<()> {
    let payload = serde_json::to_value(email).expect("email serializes");
    jobs::enqueue(conn, JOB_KIND, payload).await?;
    Ok(())
}

/// Delivers queued emails.
#[derive(Clone)]
pub enum Mailer {
    Smtp {
        transport: Box<AsyncSmtpTransport<Tokio1Executor>>,
        from: String,
    },
    /// No mail server configured: write each email to the log (development).
    Log,
}

impl Mailer {
    pub fn from_config(config: &Config) -> anyhow::Result<Self> {
        match &config.smtp_url {
            Some(url) => Ok(Self::Smtp {
                transport: Box::new(AsyncSmtpTransport::<Tokio1Executor>::from_url(url)?.build()),
                from: config.mail_from.clone(),
            }),
            None => {
                tracing::warn!("SMTP_URL is not set; emails will be logged instead of sent");
                Ok(Self::Log)
            }
        }
    }

    pub async fn send(&self, email: Email) -> anyhow::Result<()> {
        match self {
            Self::Smtp { transport, from } => {
                let message = Message::builder()
                    .from(from.parse()?)
                    .to(email.to.parse()?)
                    .subject(email.subject)
                    .header(ContentType::TEXT_PLAIN)
                    .body(email.body)?;
                transport.send(message).await?;
            }
            Self::Log => {
                tracing::info!(to = %email.to, subject = %email.subject, "email (not sent):\n{}", email.body);
            }
        }
        Ok(())
    }
}

/// Register the `send_email` handler on a worker.
pub fn register(worker: jobs::Worker, mailer: Mailer) -> jobs::Worker {
    worker.handle(JOB_KIND, move |payload| {
        let mailer = mailer.clone();
        async move {
            let email: Email = serde_json::from_value(payload)?;
            mailer.send(email).await
        }
    })
}
