use std::error::Error;
use tonic::{
    codegen::InterceptedService,
    metadata::{Ascii, MetadataValue},
    service::Interceptor,
    transport::Channel,
    Request, Status,
};

mod plugin {
    tonic::include_proto!("plugin");
}

use plugin::{plugin_client::PluginClient, Message};

type PluginResult = Result<(), Box<dyn Error>>;

/// Generic implemenation of a gRCP client for a devzat plugin.
pub struct Client {
    client: PluginClient<InterceptedService<Channel, AuthInterceptor>>,
}

struct AuthInterceptor {
    token: MetadataValue<Ascii>,
}

impl AuthInterceptor {
    pub fn new(token: String) -> Self {
        let token = format!("Bearer {}", token).parse().unwrap();
        Self { token }
    }
}

impl Interceptor for AuthInterceptor {
    fn call(&mut self, mut request: tonic::Request<()>) -> Result<tonic::Request<()>, Status> {
        request
            .metadata_mut()
            .insert("authorization", self.token.clone());
        Ok(request)
    }
}

impl Client {
    pub async fn new<S: Into<String>>(host: S, token: S) -> Result<Self, Box<dyn Error>> {
        let channel = Channel::from_shared(host.into())?.connect().await?;
        let auth = AuthInterceptor::new(token.into());
        let client = PluginClient::with_interceptor(channel, auth);

        Ok(Self { client })
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
    /// let mut client = Client::new(
    ///     "https://devzat.hackclub.com:5556",
    ///     "dvz.token@hello.world1234",
    /// );
    ///
    /// client
    /// .send_message(
    ///     String::from("#main"),
    ///     String::from("Rusty"),
    ///     String::from("Hello World from Rust!"),
    ///     None,
    /// ).await?;
    /// ```
    ///
    pub async fn send_message(
        &mut self,
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

        self.client.send_message(req).await?;

        Ok(())
    }

    // TODO: docs
    pub async fn register_listener<F>(&mut self, listener: F) -> PluginResult
    where
        F: tonic::IntoStreamingRequest<Message = plugin::ListenerClientData>,
    {
        self.client.register_listener(listener).await?;
        Ok(())
    }

    // TODO: docs
    pub async fn register_cmd<F>(&mut self, command: F) -> PluginResult
    where
        F: tonic::IntoRequest<plugin::CmdDef>,
    {
        self.client.register_cmd(command).await?;
        Ok(())
    }
}
