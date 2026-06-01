import re

with open('/home/philip/githome/robot-task-board/src/service.rs', 'r') as f:
    code = f.read()

# Add use statement
if "use robot_kanban::request_support::TeaqlUserContextExt;" not in code:
    code = code.replace("use robot_kanban::Q;", "use robot_kanban::Q;\nuse robot_kanban::request_support::TeaqlUserContextExt;")

# Add commit to add_task
code = code.replace(
    "task.save(&self.ctx).await.map_err(|e| Box::new(e) as Box<dyn Error>)?;\n\n        self.log_info(&format!(\"Finished business action: Create task '{}'\", name));",
    "task.save(&self.ctx).await.map_err(|e| Box::new(e) as Box<dyn Error>)?;\n        self.ctx.commit_data().await.map_err(|e| Box::new(e) as Box<dyn Error>)?;\n\n        self.log_info(&format!(\"Finished business action: Create task '{}'\", name));"
)

# Add commit to delete_task
code = code.replace(
    "task.delete();\n            task.save(&self.ctx).await.map_err(|e| Box::new(e) as Box<dyn Error>)?;\n            Ok(true)",
    "task.delete();\n            task.save(&self.ctx).await.map_err(|e| Box::new(e) as Box<dyn Error>)?;\n            self.ctx.commit_data().await.map_err(|e| Box::new(e) as Box<dyn Error>)?;\n            Ok(true)"
)

# Add commit to move_task
code = code.replace(
    "task.update_status_id(new_status);\n                    task.save(&self.ctx).await.map_err(|e| Box::new(e) as Box<dyn Error>)?;\n\n                    let status_name = match new_status {",
    "task.update_status_id(new_status);\n                    task.save(&self.ctx).await.map_err(|e| Box::new(e) as Box<dyn Error>)?;\n                    self.ctx.commit_data().await.map_err(|e| Box::new(e) as Box<dyn Error>)?;\n\n                    let status_name = match new_status {"
)

with open('/home/philip/githome/robot-task-board/src/service.rs', 'w') as f:
    f.write(code)

