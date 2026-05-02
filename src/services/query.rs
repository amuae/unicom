use anyhow::Result;
use tracing::info;

use crate::models::user::User;
use crate::services::unicom::{UnicomService, FlowData, BalanceData};
use crate::AppState;

pub struct QueryResult {
    pub flow_data: FlowData,
    pub balance_data: Option<BalanceData>,
    pub cookie: String,
    pub cookie_created_at: String,
    pub need_update_cookie: bool,
}

pub async fn query_user_flow(
    state: &AppState,
    user: &User,
) -> Result<QueryResult> {
    let unicom_service = UnicomService::new(state.config.unicom.clone())?;
    
    let mut cookie = user.cookie.clone();
    let mut cookie_created_at = user.cookie_created_at
        .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default();
    let mut need_login = false;

    // 判断是否需要登录
    if !user.appid.is_empty() && !user.token_online.is_empty() {
        if cookie.is_empty() {
            need_login = true;
            info!("Cookie为空，需要登录");
        } else if unicom_service.should_refresh_cookie(&cookie_created_at) {
            need_login = true;
            info!("Cookie即将过期，主动刷新");
        }
    } else if cookie.is_empty() {
        anyhow::bail!("Cookie为空且缺少登录凭证");
    }

    // 登录获取新 Cookie
    if need_login {
        let login_result = unicom_service.login(&user.appid, &user.token_online).await?;
        cookie = login_result.cookie;
        cookie_created_at = login_result.created_at;
    }

    // 查询流量
    let flow_data = match unicom_service.query_flow(&cookie).await {
        Ok(data) => data,
        Err(e) => {
            if e.to_string().contains("Cookie已失效") && !user.appid.is_empty() {
                info!("Cookie失效，重新登录");
                let login_result = unicom_service.login(&user.appid, &user.token_online).await?;
                cookie = login_result.cookie;
                cookie_created_at = login_result.created_at;
                unicom_service.query_flow(&cookie).await?
            } else {
                return Err(e);
            }
        }
    };

    // 查询余额
    let balance_data = unicom_service.query_balance(&cookie).await.ok();

    Ok(QueryResult {
        flow_data,
        balance_data,
        cookie,
        cookie_created_at,
        need_update_cookie: need_login,
    })
}
