// TODO 5: Decide if this should be updated or dropped
use crate::db_types::DbPool;
use actix_web::{web, HttpResponse};
use std::error::Error;
use tracing::error;

pub async fn status(pool: web::Data<DbPool>) -> HttpResponse {
    let mut result = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8">
    <style>
        td:first-child {
            font-weight: bold
        }
        
        table, td, th {
            border: 1px solid;
            padding: 5px;
        }
        
        table {
            border-collapse: collapse;
        }
        
        th {
            text-align: left;
        }
        
        .red {
            color: red;
        }
    </style>
    <title>Status</title>
</head>
<body>
<table>
    <tr>
        <th>System</th>
        <th>Status</th>
        <th>Message</th>
    </tr>"#
        .to_string();

    // Acquire a connection to the database to test it
    result += &format_status_row("Connect to Database", pool.acquire().await);

    // Close body and html tags
    result += "</body>
</html>";

    HttpResponse::Ok().body(result)
}

fn format_status_row<T, E: Error>(name: &str, result: Result<T, E>) -> String {
    let (stat, msg) = if let Err(e) = result {
        error!("Error for {name:?} - {e}");
        ("<span class = red>Error</span>", e.to_string())
    } else {
        ("Ok", "".to_string())
    };
    format!("<tr><td>{name}</td><td>{stat}</td><td>{msg}</td></tr>")
}
