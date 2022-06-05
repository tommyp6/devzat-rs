use std::error::Error;
use tonic::{metadata::MetadataValue, transport::Channel, Request};

mod plugin {
    tonic::include_proto!("plugin");
}

use plugin::{plugin_client::PluginClient, Message};

type PluginResult = Result<(), Box<dyn Error>>;

/// Generic implemenation of a gRCP client for a devzat plugin.
pub struct Client<T> {
    /// URL of the gRCP server handling client connections.
    pub host: String,
    /// Authorization token to connect to the gRCP server.
    pub token: String,
    client: Option<T>,
}

impl<T: crate::plugin::plugin_server::Plugin> Client<T> {
    pub fn new<S: Into<String>>(host: S, token: S) -> Self {
        Self {
            host: host.into(),
            token: token.into(),
            client: None,
        }
    }

    pub async fn connect(&mut self) -> PluginResult {
        let channel = Channel::from_shared(self.host.clone())?.connect().await?;
        let token: MetadataValue<_> = format!("Bearer {}", self.token).parse()?;
        let conn = PluginClient::with_interceptor(channel, move |mut req: Request<()>| {
            req.metadata_mut().insert("authorization", token.clone());
            Ok(req)
        });

        self.client = Some(conn);

        Ok(())
    }

    /// # Arguments
    ///
    /// `room` - Chatroom where to send the message. In devzat the default room is `#main`.
    ///
    /// `from` - This is the username the message will be sent from.
    ///
    /// `msg` - This is the actual massage that will be sent.
    ///
    /// `ephemeral_to` - This allows to send the message to a specific user.
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// let client = Client::new(
    ///     "https://devzat.hackclub.com:5556",
    ///     "dvz.token@hello.world1234",
    /// );
    ///
    /// client.connect()?; // Try to stablish a connection to the server.
    ///
    /// client.send_message(
    ///     String::from("#main"),
    ///     String::from("Rusty"),
    ///     String::from("Hello World from Rust!"),
    ///     None,
    /// )?;
    /// ```
    ///
    pub async fn send_message(
        &self,
        room: String,
        from: Option<String>,
        msg: String,
        ephemeral_to: Option<String>,
    ) -> PluginResult {
        let req = Request::new(Message {
            room,
            from,
            msg,
            ephemeral_to,
        });

        if let Some(client) = &self.client {
            client.send_message(req).await?;
        }

        Ok(())
    }
}
