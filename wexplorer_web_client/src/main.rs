use std::error::Error;

use leptos::*;
use tonic_web_wasm_client::Client;
use wexplorer_web_app_grpc_client::{web_search_response, web_searching_api_client::WebSearchingApiClient, WebSearchRequest};

async fn search(search_text: String) -> Result<Vec<web_search_response::Result>, Box<dyn Error>> {
    let mut client = WebSearchingApiClient::new(Client::new("http://localhost:8081".to_string()));
    let response = client.search(WebSearchRequest { text: search_text }).await?;
    Ok(response.into_inner().results)
}

fn main() {
    mount_to_body(|| {
        let search_action = create_action(|search_text: &String| {
            let search_text = search_text.clone();
            async move { search(search_text).await.unwrap_or_else(|e| vec![]) }
        });
        let search_results = search_action.value();

        let (search_text, set_search_text) = create_signal(String::new());

        view! {
            <input type="text" prop:value=search_text on:input=move |ev| set_search_text(event_target_value(&ev)) />
            <button on:click=move |_| search_action.dispatch(search_text.get())>"Search"</button>
            <div>
                {move || search_results.get().unwrap_or(vec![]).iter()
                    .map(|r| view! {
                        {r.entries.iter().map(|e| view! { <div>{&e.text}</div> }).collect_view()}
                        <div><a href=&r.url>{&r.url}</a></div>
                    })
                    .collect_view()}
            </div>
        }
    })
}
