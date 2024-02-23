mod repo;

use std::{env, i64, i8, sync::Arc};

use axum::{
    extract::Path,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use repo::PostgresRepo;
use serde::{Deserialize, Serialize};

#[derive(Serialize, sqlx::FromRow)]
pub struct AccountS {
    #[serde(rename = "saldo")]
    pub balance: i64,
    #[serde(rename = "data_extrato")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "limite")]
    pub limit: i64,
}

#[derive(Deserialize, Serialize, sqlx::FromRow, Clone)]
pub struct Transaction {
    #[serde(rename = "valor")]
    pub value: i64,
    #[serde(rename = "tipo")]
    pub kind: TransactionType,
    #[serde(rename = "descricao")]
    pub description: String,
    #[serde(rename = "realizada_em", default = "created_now")]
    pub created_at: DateTime<Utc>,
}
#[derive(Deserialize, Serialize, sqlx::Type, Clone, Copy)]
pub enum TransactionType {
    #[serde(rename = "c")]
    Credit,
    #[serde(rename = "d")]
    Debit,
}
#[derive(Serialize, sqlx::FromRow)]
pub struct Account {
    limit: i64,
    balance: i64,
}

#[derive(Serialize)]
pub struct BankStatement {
    acs: AccountS,
    #[serde(rename = "ultimas_transacoes")]
    last_transactions: Vec<Transaction>,
}

fn created_now() -> DateTime<Utc> {
    Utc::now()
}

type AppState = Arc<PostgresRepo>;

#[tokio::main]
async fn main() {
    // build our application with a single route
    let url = env::var("DATABASE_URL").unwrap_or(String::from("DATABASE_URL"));
    let repo = PostgresRepo::connect(url);
    let appstate = Arc::new(repo.await);
    let app = Router::new()
        .route("/clientes/:id/extrato", get(view_bankstatemant))
        .route("/clientes/:id/extrato", post(create_transaction))
        .with_state(appstate);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}



async fn view_bankstatemant(
    State(db): State<AppState>,
    Path(client_id): Path<i8>,
) -> impl IntoResponse {
if !(1..=5).contains(&client_id){
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }

    match db.get_bank_statement(&client_id).await {
        Ok(Some(bs)) => Ok(Json(bs)),
        Ok(None) =>Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR)
        
    }
    
}
async fn create_transaction(
    State(db): State<AppState>,
    Path(client_id): Path<i8>,
    Json(mut transaction): Json<Transaction>,
) -> impl IntoResponse {
     if !(1..=5).contains(&client_id){
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }

    if transaction.description.is_empty()  {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }

    if transaction.description != "c" && transaction.description != "d"{
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }

    if transaction.description.len() > 10 {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }

    if transaction.value <= 0 {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    
    match db.create_transaction(&mut transaction,&client_id).await {
       
        Ok(Some(account)) => Ok(Json(account)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => match e {
            sqlx::Error::ColumnNotFound(_) => Err(StatusCode::UNPROCESSABLE_ENTITY),
            _ => Err(StatusCode::INTERNAL_SERVER_ERROR)
            
        },
    }
}
