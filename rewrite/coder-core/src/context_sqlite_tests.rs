#[cfg(test)]
mod tests {
    use crate::context_sqlite::SqliteContext;

    #[tokio::test]
    async fn sqlite_context_push_and_recent() {
        let ctx = SqliteContext::new_in_memory().unwrap();
        ctx.push("a".to_string()).await.unwrap();
        ctx.push("b".to_string()).await.unwrap();

        let recent1 = ctx.recent(1).await.unwrap();
        assert_eq!(recent1, vec!["b".to_string()]);

        let recent2 = ctx.recent(5).await.unwrap();
        assert_eq!(recent2, vec!["a".to_string(), "b".to_string()]);
    }
}
