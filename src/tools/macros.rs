//! 工具宏定义
//!
//! 提供便捷的工具注册和调用宏

/// 注册工具宏
#[macro_export]
macro_rules! register_tool {
    ($registry:expr, $name:expr, $desc:expr, $func:expr) => {
        $registry.register($crate::tools::ToolDefinition {
            name: $name,
            description: $desc,
            function: std::sync::Arc::new($func),
        });
    };
}

/// 定义工具宏
#[macro_export]
macro_rules! define_tool {
    (
        name: $name:expr,
        desc: $desc:expr,
        fn $func_name:ident($($arg_name:ident: $arg_type:ty),*) -> $ret_type:ty $body:block
    ) => {
        pub fn $func_name($($arg_name: $arg_type),*) -> $ret_type $body
        
        $crate::register_tool!(
            registry,
            $name,
            $desc,
            |args| {
                // 自动参数提取和调用
                $crate::extract_args!(args, $($arg_name: $arg_type),*);
                Ok(serde_json::to_value($func_name($($arg_name),*))?)
            }
        );
    };
}

/// 提取参数宏
#[macro_export]
macro_rules! extract_args {
    ($args:expr, $($name:ident: $type:ty),*) => {
        $(
            let $name = $args[stringify!($name)]
                .as_ref()
                .and_then(|v| serde_json::from_value::<$type>(v.clone()).ok())
                .ok_or_else(|| $crate::tools::ToolError::InvalidArgs(
                    format!("缺少参数或类型错误：{}", stringify!($name))
                ))?;
        )*
    };
}

/// 工具调用宏
#[macro_export]
macro_rules! call_tool {
    ($registry:expr, $name:expr, $($arg_name:ident: $arg_value:expr),*) => {
        $registry.call($name, serde_json::json!({
            $(stringify!($arg_name): $arg_value),*
        }))
    };
}

/// 批量注册工具宏
#[macro_export]
macro_rules! register_tools {
    ($registry:expr; $($name:expr => $func:expr),* $(,)?) => {
        $(
            $registry.register($crate::tools::ToolDefinition {
                name: $name,
                description: "",
                function: std::sync::Arc::new($func),
            });
        )*
    };
}
