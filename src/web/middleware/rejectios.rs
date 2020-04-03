use std::pin::Pin;
use std::task::{Context, Poll};

use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    http::header::USER_AGENT,
    Error,
};
use futures::future::{ok, Ready};
use futures::Future;

#[derive(Debug, Default)]
pub struct RejectOldIos;

impl RejectOldIos {
    pub fn new() -> Self {
        RejectOldIos::default()
    }
}

impl<S, B> Transform<S> for RejectOldIos
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = RejectOldIosMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(RejectOldIosMiddleware { service })
    }
}

pub struct RejectOldIosMiddleware<S> {
    service: S,
}

impl<S, B> Service for RejectOldIosMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, sreq: ServiceRequest) -> Self::Future {
        if let Some(header) = sreq.headers().get(USER_AGENT) {
            if let Ok(ua) = header.to_str() {
                use actix_web::HttpResponse;
                use futures::future;
                eprintln!("Hi from start. You: {:#?} requested: {}", ua, sreq.path());
                return Box::pin(future::ok(
                    sreq.into_response(
                        HttpResponse::InternalServerError()
                            //                            .content_type("application/json")
                            .body("XXX".to_owned())
                            .into_body(),
                    ),
                ));
            }
        }

        let fut = self.service.call(sreq);

        Box::pin(async move {
            let res = fut.await?;

            eprintln!("Hi from response");
            Ok(res)
        })
    }
}
