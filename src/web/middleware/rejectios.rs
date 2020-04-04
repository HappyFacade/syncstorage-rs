use std::pin::Pin;
use std::task::{Context, Poll};

use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    http::header::USER_AGENT,
    Error, HttpResponse,
};
use futures::future::{self, Ready};
use futures::Future;
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    // e.g. "Firefox-iOS-Sync/18.0b1 (iPhone; iPhone OS 13.2.2) (Fennec (synctesting))"
    static ref IOS_REGEX: Regex = Regex::new(
        r"(?x)
^
Firefox-iOS-Sync/
(?P<major>[0-9]+)\.[.0-9]+    # <appVersion-major>.<appVersion-minor-etc>
b([0-9]+)                     # b<builderNumber>
\s\([[:word:]]+               #  (<deviceModel>
;\siPhone\sOS                 # ; iPhone OS
\s[.0-9]+\)                   #  <systemVersion>)
\s\(.*\)                      #  (<displayName>)
$
"
    )
    .unwrap();
}

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
        future::ok(RejectOldIosMiddleware { service })
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
                if should_reject(ua) {
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
        }
        self.service.call(sreq)
/*
        let fut = self.service.call(sreq);

        Box::pin(async move {
            let res = fut.await?;

            eprintln!("Hi from response");
            Ok(res)
        })
*/
    }
}

fn should_reject(ua: &str) -> bool {
    if let Some(captures) = IOS_REGEX.captures(ua) {
        if let Some(major) = captures.name("major") {
            let major = major.as_str().parse::<u32>().unwrap_or(20);
            return major < 20;
        }
    }
    false
}
