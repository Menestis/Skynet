use crate::AppData;
use crate::database::servers::Server;
use crate::web::rejections::ApiError;

pub async fn handle_property_actions(_data: &AppData, _srv: &Server) -> Result<(), ApiError>{

    Ok(())
}