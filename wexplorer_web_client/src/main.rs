use std::error::Error;

use leptos::*;
use tonic_web_wasm_client::Client;
use wexplorer_web_app_grpc_client::{echo_service_client::EchoServiceClient, EchoRequest};

async fn send_echo(msg: &str) -> Result<String, Box<dyn Error>> {
    let mut client = EchoServiceClient::new(Client::new("http://localhost:8081".to_string()));
    let response = client.echo(EchoRequest { message: msg.to_string() }).await?;
    Ok(response.get_ref().message.to_string())
}

fn main() {
    mount_to_body(|| {
        let send_echo_action = create_action(|msg: &String| {
            let msg = msg.clone();
            async move { send_echo(&msg).await.unwrap_or_else(|e| e.to_string()) }
        });
        let last_echo = send_echo_action.value();
        let (msg, set_msg) = create_signal(String::default());

        view! {
            <input type="text" prop:value=msg on:input=move |ev| set_msg(event_target_value(&ev)) />
            <button on:click=move |_| send_echo_action.dispatch(msg.get())>"Send echo"</button>
            <div>
                "Last echo: "
                {move || last_echo.get()}
            </div>
        }
    })
}
