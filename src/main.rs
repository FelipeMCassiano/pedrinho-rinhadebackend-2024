mod repo;

use serde_json::json;
use std::{env, sync::Arc};

use axum::{
    body::Body,
    extract::Path,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    response::Response,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use repo::PostgresRepo;
use serde::{Deserialize, Serialize};

#[derive(Serialize, sqlx::FromRow)]
pub struct AccountS {
    pub total: i32,
    pub data_extrato: DateTime<Utc>,
    pub limite: i32,
}

#[derive(Deserialize, Serialize, sqlx::FromRow, Clone)]
pub struct Transaction {
    pub valor: i32,
    pub tipo: TransactionType,
    pub descricao: String,
    #[serde(default = "created_now")]
    pub realizada_em: DateTime<Utc>,
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
    #[serde(rename = "limite")]
    limit: i32,
    #[serde(rename = "saldo")]
    balance: i32,
}
#[derive(Serialize)]
pub struct TransactionDb {
    valor : i32,
    descricao: String,
    realizada_em: DateTime<Utc>,
    tipo: String
}

#[derive(Serialize)]
pub struct BankStatement {
    #[serde(rename = "saldo")]
    saldo: AccountS,
    #[serde(rename = "ultimas_transacoes")]
    last_transactions: Vec<TransactionDb>,
}

fn created_now() -> DateTime<Utc> {
    Utc::now()
}

type AppState = Arc<PostgresRepo>;

#[tokio::main]
async fn main() {
    // build our application with a single route
    let url = env::var("DATABASE_URL").unwrap_or(String::from(
        "postgres://rinha:rinha@localhost:5432/rinha?sslmode=disable",
    ));

    // let url = env::var("DATABASE_URL").unwrap_or(String::from("postgres://rinha:rinha@db:5432/rinha?sslmode=disable"));
    let repo = PostgresRepo::connect(url);
    let appstate = Arc::new(repo.await);
    let app = Router::new()
        .route("/clientes/:id/extrato", get(view_bankstatemant))
        .route("/clientes/:id/transacoes", post(create_transaction))
        .with_state(appstate);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn view_bankstatemant(
    State(db): State<AppState>,
    Path(client_id): Path<i32>,
) -> impl IntoResponse {
    if !(1..=5).contains(&client_id) {
        return Err(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from(
                json!({"error": "Invalid client ID"}).to_string(),
            ))
            .unwrap());
    }

    match db.get_bank_statement(&client_id).await {
        Ok(Some(bs)) => Ok(Json(bs)),
        Ok(None) => Err(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from(
                json!({"error": "Bank statement not found"}).to_string(),
            ))
            .unwrap()),
        Err(e) => Err(Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from(json!({"error": e.to_string()}).to_string()))
            .unwrap()),
    }
}
async fn create_transaction(
    State(db): State<AppState>,
    Path(client_id): Path<i32>,
    Json(mut transaction): Json<Transaction>,
) -> impl IntoResponse {
    println!("comecei a criar");
    if !(1..=5).contains(&client_id) {
        return Err(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from(json!({"error": "client not foud"}).to_string()))
            .unwrap());
    }
    println!("passei aq no handler");

    if transaction.descricao.is_empty() {
        return Err(Response::builder()
            .status(StatusCode::UNPROCESSABLE_ENTITY)
            .body(Body::from(
                json!({"error": "blank description"}).to_string(),
            ))
            .unwrap());
    }

    if transaction.descricao.len() > 10 {
        return Err(Response::builder()
            .status(StatusCode::UNPROCESSABLE_ENTITY)
            .body(Body::from(
                json!({"error": "description too big"}).to_string(),
            ))
            .unwrap());
    }

    if transaction.valor <= 0 {
        return Err(Response::builder()
            .status(StatusCode::UNPROCESSABLE_ENTITY)
            .body(Body::from(
                json!({"error": "Transaction value needs to be bigger than 0"}).to_string(),
            ))
            .unwrap());
    }

    match db.create_transaction(&mut transaction, &client_id).await {
        Ok(Some(account)) => Ok(Json(account)),
        Ok(None) => Err(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from(
                json!({"error": " clinet not foudn 2"}).to_string(),
            ))
            .unwrap()),
        Err(e) => match e {
            sqlx::Error::ColumnNotFound(_) => Err(Response::builder()
                .status(StatusCode::UNPROCESSABLE_ENTITY)
                .body(Body::from(json!({"error": "limit exceed"}).to_string()))
                .unwrap()),
            _ => Err(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(json!({"error": e.to_string()}).to_string()))
                .unwrap()),
        },
    }
}
