use sqlx::{postgres::PgPoolOptions, query, query_as, Error, PgPool, Row};
use std::mem;

use crate::{Account, AccountS, BankStatement, Transaction};

pub struct PostgresRepo {
    pool: PgPool,
}

impl PostgresRepo {
    pub async fn connect(url: String) -> Self {
        PostgresRepo {
            pool: PgPoolOptions::new()
                .max_connections(16)
                .connect(&url)
                .await
                .unwrap(),
        }
    }
    pub async fn create_transaction(
        &self,
        transaction: &mut Transaction,
        cliente_id: &i8,
    ) -> Result<Option<Account>, Error> {
        let mut tx = self.pool.begin().await?;
        let account: Account =
            sqlx::query_as("SELECT limite,saldo FROM clientes WHERE id = $1 FOR UPDATE")
                .bind(cliente_id)
                .fetch_one(&mut *tx)
                .await?;

        let kind_t = transaction.kind;

        let t = Transaction {
            value: transaction.value,
            description: mem::take(&mut transaction.description),
            created_at: transaction.created_at,
            kind: kind_t,
        };
        let newbalance = match kind_t {
            crate::TransactionType::Credit => account.balance + transaction.value,
            crate::TransactionType::Debit => account.balance - transaction.value,
        };

        if (account.limit + newbalance) < 0 {
            return Err(Error::ColumnNotFound(String::from(
                "Account limit exceeded",
            )));
        }

        sqlx::query(
        "INSERT INTO transacoes (cliente_id, valor, tipo, descricao, realizada_em) VALUES($1, $2, $3, $4,$5)"
    ).bind(cliente_id).bind(t.value).bind(t.kind).bind(t.description).bind(t.created_at).execute(&mut *tx).await?;
        sqlx::query(
            "
        UPDATE clientes SET saldo=$1 WHERE id=$2
        ",
        )
        .bind(newbalance)
        .bind(cliente_id)
        .execute(&mut *tx)
        .await?;

        let a = Account {
            limit: account.limit, balance: newbalance, };
        tx.commit().await?;

        Ok(Some(a))
    }

    pub async fn get_bank_statement(&self, id: &i8) -> Result<Option<BankStatement>, sqlx::Error> {
        let account_s: AccountS =
            query_as("SELECT saldo, now(), limite FROM clientes WHERE id = $1")
                .bind(id)
                .fetch_one(&self.pool)
                .await?;

        let queryresutl = query("SELECT valor, tipo, descricao, realizada_em FROM transacoes WHERE cliente_id=$1 ORDER BY realizada_em DESC LIMIT 10").bind(id).fetch_all(&self.pool).await;
        let mut vtransaction = Vec::new();
        match queryresutl {
            Ok(q) => {
                for r in q {
                    vtransaction.push(Transaction {
                        value: r.try_get("valor")?,
                        kind: r.try_get("tipo")?,
                        description: r.try_get("descricao")?,
                        created_at: r.try_get("realizada_em")?,
                    });
                }
                Ok(Some(BankStatement {
                    acs: account_s,
                    last_transactions: vtransaction,
                }))
            }
            Err(e) => Err(e),
        }
    }
}
