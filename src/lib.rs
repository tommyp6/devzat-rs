use futures_util::stream;
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

use plugin::{
    listener_client_data::Data, plugin_client::PluginClient, CmdDef, CmdInvocation, Event,
    ListenerClientData, Message, MiddlewareResponse,
};

pub use plugin::Listener;

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
    ///     .send_message(
    ///         String::from("#main"),
    ///         String::from("Rusty"),
    ///         String::from("Hello World from Rust!"),
    ///         None,
    ///     )
    ///     .await?;
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

    /// # Arguments
    ///
    /// `listener` - [Listener] struct containing information about the listener.
    ///
    /// `callback` - Asynchronous function to be executed.
    ///
    /// # Examples
    ///
    /// ```
    /// let listener = Listener {
    ///     middleware: None,
    ///     once: None,
    ///     regex: None,
    /// };
    ///
    /// client
    ///     .register_listener(listener, |event| move async {
    ///         eprintln!("room={}, from={}, msg={}", event.room, event.from, event.msg);
    ///     })
    ///     .await?;
    /// ```
    ///
    pub async fn register_listener<F, Fut>(
        &mut self,
        listener: Listener,
        callback: F,
    ) -> PluginResult
    where
        F: FnOnce(Event) -> Fut + Copy,
        Fut: std::future::Future<Output = Option<String>>,
    {
        let listener_data = stream::iter(vec![ListenerClientData {
            data: Some(Data::Listener(listener.clone())),
        }]);

        let mut event = self
            .client
            .register_listener(listener_data)
            .await?
            .into_inner();

        while let Some(event) = event.message().await? {
            let result = callback(event).await;

            if !listener.middleware() && result.is_some() {
                panic!("Function returned a value although it's not marked as a middleware.");
            }

            // TBD: Send/Write this? How?
            // https://github.com/Merlin04/devzat-node/blob/be29a311371b2d7c9814e5dc6cda3a955a8cf628/src/index.ts#L108

            Data::Response(MiddlewareResponse { msg: result });
        }

        Ok(())
    }

    /// # Arguments
    ///
    /// `name` - Command name.
    ///
    /// `info` - Command information.
    ///
    /// `args_info` - Information about the command arguments.
    ///
    /// `callback` - Asynchronous function that will be ran on command invocation.
    ///
    /// # Examples
    ///
    /// ```
    /// client
    ///     .register_cmd("greet", "Greet someone.", "<name>", |event| async move {
    ///         format!("Hello {}!", event.args)
    ///     })
    ///     .await?;
    /// ```
    ///
    pub async fn register_cmd<S, F, Fut>(
        &mut self,
        name: S,
        info: S,
        args_info: S,
        callback: F,
    ) -> PluginResult
    where
        S: Into<String>,
        F: FnOnce(CmdInvocation) -> Fut + Copy,
        Fut: std::future::Future<Output = String>,
    {
        let cmd = CmdDef {
            name: name.into(),
            info: info.into(),
            args_info: args_info.into(),
        };

        let mut event = self.client.register_cmd(cmd).await?.into_inner();

        while let Some(event) = event.message().await? {
            let room = event.room.clone();
            let result = callback(event).await;
            self.send_message(room, None, result, None).await?;
        }

        Ok(())
    }
}
