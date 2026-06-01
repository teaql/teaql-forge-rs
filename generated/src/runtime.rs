pub struct Context {
    // Database connection would go here
}

pub async fn connect_sqlite(_url: &str) -> Result<Context, String> {
    Ok(Context {})
}

impl Context {
    pub async fn save<T>(&self, _entity: T) -> Result<T, String> {
        Ok(_entity)
    }
    
    pub async fn delete<T>(&self, _entity: T) -> Result<(), String> {
        Ok(())
    }
}