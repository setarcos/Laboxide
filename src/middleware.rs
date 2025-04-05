use std::future::{Ready, ready};
use actix_web::{
    dev::{self, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse, body::EitherBody,
};
use serde_json::json;
use actix_session::SessionExt;
use futures_util::future::LocalBoxFuture;

pub struct CheckPermission {
    perm: i64,
}

impl CheckPermission {
    pub fn new(required: i64) -> Self {
        CheckPermission { perm: required }
    }
}

impl<S, B> Transform<S, ServiceRequest> for CheckPermission
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = CheckPermissionMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(CheckPermissionMiddleware { service, perm: self.perm }))
    }
}

pub struct CheckPermissionMiddleware<S> {
    service: S,
    perm: i64,
}

impl<S, B> Service<ServiceRequest> for CheckPermissionMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    dev::forward_ready!(service);

    fn call(&self, request: ServiceRequest) -> Self::Future {

        // Check if user has admin permission (assuming '1' represents admin permission)
        if let Ok(Some(permission)) = request.get_session().get::<i64>("permissions"){
            let required = self.perm;
            if permission & required != 0 {
                // User has admin permission, proceed with the request
                let res = self.service.call(request);
                Box::pin(async move {
                    // Forward the response
                    res.await.map(ServiceResponse::map_into_left_body)
                })
            } else {
                // User does not have admin permission, return Forbidden response
                let (request, _pl) = request.into_parts();
                let response = HttpResponse::Forbidden()
                    .json(json!({"Forbidden": "Admin permission required"}))
                    .map_into_right_body();

                Box::pin(async { Ok(ServiceResponse::new(request, response)) })
            }
        } else {
            // User is not authenticated or permission is not found, return Unauthorized response
            let (request, _pl) = request.into_parts();
            let response = HttpResponse::Unauthorized()
                .json(json!({"Unauthorized": "User not logged in"}))
                .map_into_right_body();

            Box::pin(async { Ok(ServiceResponse::new(request, response)) })
        }
    }
}

