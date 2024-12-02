// use std::net::SocketAddr;

// use http_body_util::Full;
// use hyper::body::Bytes;
// use hyper::server::conn::http1;
// use hyper::service::service_fn;
// use hyper::{Response, StatusCode};
// use hyper_util::rt::TokioIo;
// use tauri::{AppHandle, Runtime};
// use tokio::net::TcpListener;

use holochain_runtime::FileSystem;

pub fn pong_iframe() -> String {
    format!("<html><head></head><body><script>window.onload = () => window.parent.postMessage('pong', '*') </script></body></html>")
}

// pub fn window_html() -> String {
//     include_str!("../../packages/frame/dist/index.html").into()
// }

// pub async fn start_http_server<R: Runtime>(
//     app_handle: AppHandle<R>,
//     ui_server_port: u16,
// ) -> crate::Result<()> {
//     let addr: SocketAddr = ([127, 0, 0, 1], ui_server_port).into();
//     let listener = TcpListener::bind(addr).await?;
//     tauri::async_runtime::spawn(async move {
//         let app_handle = app_handle.clone();
//         // The closure inside `make_service_fn` is run for each connection,
//         // creating a 'service' to handle requests for that specific connection.
//         // let make_service = make_service_fn(move |_| async {
//         //     // While the state was moved into the make_service closure,
//         //     // we need to clone it here because this closure is called
//         //     // once for every connection.
//         //     //
//         //     // Each connection could send multiple requests, so
//         //     // the `Service` needs a clone to handle later requests.
//         //     // This is the `Service` that will handle the connection.
//         //     // `service_fn` is a helper to convert a function that
//         //     // returns a Response into a `Service`.
//         //     // let app_handle = app_handle.clone();
//         //     Ok::<_, hyper::Error>(service_fn(move |request| {
//         //         let app_handle = app_handle.clone();
//         //         async move {}
//         //     }))
//         // });

//         loop {
//             let app_handle = app_handle.clone();
//             if let Ok((stream, _)) = listener.accept().await {
//                 let io = TokioIo::new(stream);

//                 // This is the `Service` that will handle the connection.
//                 // `service_fn` is a helper to convert a function that
//                 // returns a Response into a `Service`.
//                 let service = service_fn(move |request| {
//                     let app_handle = app_handle.clone();
//                     async move {
//                         let host = request
//                             .headers()
//                             .get("host")
//                             .expect("URI has no host")
//                             // .ok_or(crate::Error::HttpServerError(String::from(
//                             //     "URI has no host",
//                             // )))?
//                             .clone()
//                             .to_str()
//                             .expect("Could not convert str to string")
//                             // .map_err(|err| crate::Error::HttpServerError(format!("{:?}", err)))?
//                             .to_string();

//                         if host.starts_with("ping.localhost") {
//                             return Response::builder()
//                                 .status(202)
//                                 .header("content-type", "text/html")
//                                 .body(pong_iframe().into());
//                         }
//                         if host.starts_with("localhost") {
//                             return Response::builder()
//                                 .status(202)
//                                 .header("content-type", "text/html")
//                                 .body(window_html().into());
//                         }

//                         let split_host: Vec<String> =
//                             host.split(".").into_iter().map(|s| s.to_string()).collect();
//                         let lowercase_app_id = split_host.get(0).expect("Failed to get the app id");

//                         let file_name = request.uri().path();

//                         let Ok(holochain) = app_handle.holochain() else {
//                             return Response::builder()
//                                 .status(StatusCode::INTERNAL_SERVER_ERROR)
//                                 .body(
//                                     format!("Called http UI before initializing holochain")
//                                         .as_bytes()
//                                         .to_vec()
//                                         .into(),
//                                 );
//                         };

//                         let r: hyper::http::Result<Response<Full<Bytes>>> = match read_asset(
//                             &holochain.holochain_runtime.filesystem,
//                             &lowercase_app_id,
//                             file_name.to_string(),
//                         )
//                         .await
//                         {
//                             Ok(Some((asset, mime_type))) => {
//                                 let mut response_builder = Response::builder().status(202);
//                                 if let Some(mime_type) = mime_type {
//                                     response_builder =
//                                         response_builder.header("content-type", mime_type);
//                                 }

//                                 response_builder.body(asset.into())
//                             }
//                             Ok(None) => Response::builder().status(404).body(vec![].into()),
//                             Err(e) => Ok(Response::builder()
//                                 .status(500)
//                                 .body(format!("{:?}", e).into())
//                                 .expect("Failed to build body of error response")),
//                         };
//                         // admin_ws.close();
//                         r
//                     }
//                     // Get the current count, and also increment by 1, in a single
//                     // // atomic operation.
//                     // let count = counter.fetch_add(1, Ordering::AcqRel);
//                     // async move {
//                     //     Ok::<_, Error>(Response::new(Full::new(Bytes::from(format!(
//                     //         "Request #{}",
//                     //         count
//                     //     )))))
//                     // }
//                 });

//                 if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
//                     println!("Error serving connection: {:?}", err);
//                 }
//             } else {
//                 log::error!("Could not accept incoming tcp stream");
//             }
//         }
//     });

//     Ok(())
// }

pub async fn read_asset(
    fs: &FileSystem,
    app_id: &String,
    mut asset_name: String,
) -> crate::Result<Option<(Vec<u8>, Option<String>)>> {
    log::debug!("Reading asset from filesystem. Asset name: {}", asset_name);
    if asset_name.starts_with("/") {
        asset_name = asset_name
            .strip_prefix("/")
            .expect("Failed to strip prefix")
            .to_string();
    }
    if asset_name == "" {
        asset_name = String::from("index.html");
    }

    let assets_path = fs.bundle_store.get_ui_path(&app_id)?;
    let asset_file = assets_path.join(asset_name);

    let mime_guess = mime_guess::from_path(asset_file.clone());

    let mime_type = match mime_guess.first() {
        Some(mime) => Some(mime.essence_str().to_string()),
        None => {
            log::warn!("Could not determine MIME Type of file '{:?}'", asset_file);
            None
        }
    };

    match std::fs::read(asset_file.clone()) {
        Ok(asset) => Ok(Some((asset, mime_type))),
        Err(_e) => {
            // Fallback to "index.html" to support push-based client-side routing without hashing
            let asset_file = assets_path.join(String::from("index.html"));
            let mime_guess = mime_guess::from_path(asset_file.clone());

            let mime_type = match mime_guess.first() {
                Some(mime) => Some(mime.essence_str().to_string()),
                None => {
                    log::warn!("Could not determine MIME Type of file '{:?}'", asset_file);
                    None
                }
            };
            match std::fs::read(asset_file.clone()) {
                Ok(asset) => Ok(Some((asset, mime_type))),
                Err(_e) => Ok(None)
            }
        },
    }
}
