use chrono:: Utc;
use sqlx::{postgres::PgPoolOptions, query, Error, PgPool, Row};
use std::mem;

use crate::{Account, AccountS, BankStatement, Transaction, TransactionDb};

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
        cliente_id: &i32,
    ) -> Result<Option<Account>, Error> {
        let mut tx = self.pool.begin().await?;
        println!("passei 1 tr");
        let queryresult = sqlx::query("SELECT limite,saldo FROM clientes WHERE id = $1 FOR UPDATE")
            .bind(cliente_id)
            .fetch_one(&mut *tx)
            .await?;
        println!("passei 2 tr");
        let account = Account {
            limit: queryresult.try_get("limite")?,
            balance: queryresult.try_get("saldo")?,
        };
        let kind_t = transaction.tipo;

        let t = TransactionDb {
            valor: transaction.valor,
            descricao: mem::take(&mut transaction.descricao),
            realizada_em: transaction.realizada_em,
            tipo: match kind_t {
                crate::TransactionType::Credit => String::from("c"),
                crate::TransactionType::Debit =>  String::from("d"),
                
            },
        };
        let newbalance = match kind_t {
            crate::TransactionType::Credit => account.balance + transaction.valor,
            crate::TransactionType::Debit => account.balance - transaction.valor,
        };

        if (account.limit + newbalance) < 0 {
            return Err(Error::ColumnNotFound(String::from(
                "Account limit exceeded",
            )));
        }

        println!("passei 3 tr");
        sqlx::query(
        "INSERT INTO transacoes (cliente_id, valor, tipo, descricao, realizada_em) VALUES($1, $2, $3, $4,$5)"
    ).bind(cliente_id).bind(t.valor).bind(t.tipo).bind(t.descricao).bind(t.realizada_em).execute(&mut *tx).await?;

        sqlx::query(
            "
        UPDATE clientes SET saldo=$1 WHERE id=$2
        ",
        )
        .bind(newbalance)
        .bind(cliente_id)
        .execute(&mut *tx)
        .await?;

        println!("passei 4 tr");
        let a = Account {
            limit: account.limit,
            balance: newbalance,
        };
        tx.commit().await?;
        println!(" tr");

        Ok(Some(a))
    }

    pub async fn get_bank_statement(&self, id: &i32) -> Result<Option<BankStatement>, sqlx::Error> {
        let queryresult = query("SELECT saldo, limite FROM clientes WHERE id = $1")
            .bind(id)
            .fetch_one(&self.pool)
            .await?;

        let account_s = AccountS {
            limite: queryresult.try_get("limite")?,
            total: queryresult.try_get("saldo")?,
            data_extrato: Utc::now(),
        };
        let mut bs = BankStatement {
            saldo: account_s,
            last_transactions: Vec::new(),
        };
        println!("passei 1 e");
        let queryresutl = query("SELECT valor, tipo, descricao, realizada_em FROM transacoes WHERE cliente_id=$1 ORDER BY realizada_em DESC LIMIT 10").bind(id).fetch_all(&self.pool).await;
        let mut vtransaction: Vec<TransactionDb> = Vec::new();

        match queryresutl {
            Ok(q) => {
                for r in q {
                    vtransaction.push(TransactionDb {
                        valor: r.try_get("valor")?,
                        tipo: r.try_get("tipo")?,
                        descricao: r.try_get("descricao")?,
                        realizada_em: r.try_get("realizada_em")?,
                    });
                }
                println!("passei 2 e");
                bs.last_transactions.append(&mut vtransaction);
                Ok(Some(bs))
                
            }

            Err(e) => Err(e),
        }
    }
}
