#[cfg(test)]
mod tests {
    use crate::context::Context;

    #[tokio::test]
    async fn context_stores_messages() {
        let ctx = Context::new();
        ctx.push("first".to_string()).await;
        ctx.push("second".to_string()).await;

        let recent1 = ctx.recent(1).await;
        assert_eq!(recent1, vec!["second".to_string()]);

        let recent2 = ctx.recent(5).await;
        assert_eq!(recent2, vec!["first".to_string(), "second".to_string()]);
    }
}
