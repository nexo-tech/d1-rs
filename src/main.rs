use worker::*;
use calendar_app::worker_handlers;

#[event(fetch)]
pub async fn main(req: Request, env: Env, ctx: Context) -> Result<Response> {
    worker_handlers::handle_request(req, env, ctx).await
}