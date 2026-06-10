#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeCapability {
    FilesystemRead,
    FilesystemWrite,
    FilesystemDelete,
    ProcessExec,
    ShellExec,
    EnvRead,
    EnvWrite,
    NetworkClient,
    NetworkServer,
    Database,
    Clock,
    Random,
}

impl NativeCapability {
    pub fn as_str(self) -> &'static str {
        match self {
            NativeCapability::FilesystemRead => "filesystem-read",
            NativeCapability::FilesystemWrite => "filesystem-write",
            NativeCapability::FilesystemDelete => "filesystem-delete",
            NativeCapability::ProcessExec => "process-exec",
            NativeCapability::ShellExec => "shell-exec",
            NativeCapability::EnvRead => "env-read",
            NativeCapability::EnvWrite => "env-write",
            NativeCapability::NetworkClient => "network-client",
            NativeCapability::NetworkServer => "network-server",
            NativeCapability::Database => "database",
            NativeCapability::Clock => "clock",
            NativeCapability::Random => "random",
        }
    }

    pub fn allow_flag(self) -> &'static str {
        match self {
            NativeCapability::FilesystemRead => "--allow-fs-read",
            NativeCapability::FilesystemWrite => "--allow-fs-write",
            NativeCapability::FilesystemDelete => "--allow-fs-delete",
            NativeCapability::ProcessExec => "--allow-process-exec",
            NativeCapability::ShellExec => "--allow-shell-exec",
            NativeCapability::EnvRead => "--allow-env-read",
            NativeCapability::EnvWrite => "--allow-env-write",
            NativeCapability::NetworkClient => "--allow-net-client",
            NativeCapability::NetworkServer => "--allow-net-server",
            NativeCapability::Database => "--allow-database",
            NativeCapability::Clock => "--allow-clock",
            NativeCapability::Random => "--allow-random",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct RuntimeCapabilityPolicy {
    pub filesystem_read: bool,
    pub filesystem_write: bool,
    pub filesystem_delete: bool,
    pub process_exec: bool,
    pub shell_exec: bool,
    pub env_read: bool,
    pub env_write: bool,
    pub network_client: bool,
    pub network_server: bool,
    pub database: bool,
    pub clock: bool,
    pub random: bool,
}

impl RuntimeCapabilityPolicy {
    pub fn trusted() -> Self {
        Self {
            filesystem_read: true,
            filesystem_write: true,
            filesystem_delete: true,
            process_exec: true,
            shell_exec: true,
            env_read: true,
            env_write: true,
            network_client: true,
            network_server: true,
            database: true,
            clock: true,
            random: true,
        }
    }

    pub fn restricted() -> Self {
        Self::default()
    }

    pub fn allows(&self, capability: NativeCapability) -> bool {
        match capability {
            NativeCapability::FilesystemRead => self.filesystem_read,
            NativeCapability::FilesystemWrite => self.filesystem_write,
            NativeCapability::FilesystemDelete => self.filesystem_delete,
            NativeCapability::ProcessExec => self.process_exec,
            NativeCapability::ShellExec => self.shell_exec,
            NativeCapability::EnvRead => self.env_read,
            NativeCapability::EnvWrite => self.env_write,
            NativeCapability::NetworkClient => self.network_client,
            NativeCapability::NetworkServer => self.network_server,
            NativeCapability::Database => self.database,
            NativeCapability::Clock => self.clock,
            NativeCapability::Random => self.random,
        }
    }
}

pub fn capability_for_native_function(name: &str) -> Option<NativeCapability> {
    match name {
        // Filesystem read
        "read_file" | "read_file_sync" | "read_file_async" | "read_binary_file" | "read_lines"
        | "list_dir" | "list_dir_sync" | "list_dir_async" | "file_exists" | "file_size"
        | "path_exists" | "path_is_dir" | "path_is_file" | "path_extension" | "path_absolute"
        | "path_is_symlink" | "dirname" | "basename" | "join_path" | "path_join" | "os_getcwd"
        | "os_environ" | "io_read_bytes" | "io_read_at" | "io_seek_read" | "io_file_metadata"
        | "load_image" | "md5_file" | "sha256_file" | "read_file_lossy" | "async_read_file"
        | "async_read_files" | "kv_get" => Some(NativeCapability::FilesystemRead),

        // Filesystem write
        "write_file"
        | "write_file_sync"
        | "write_file_async"
        | "append_file"
        | "write_binary_file"
        | "create_dir"
        | "rename_file"
        | "copy_file"
        | "os_chdir"
        | "zip_create"
        | "zip_add_file"
        | "zip_add_dir"
        | "zip_close"
        | "unzip"
        | "gif_to_webp"
        | "io_write_bytes"
        | "io_append_bytes"
        | "io_write_at"
        | "io_truncate"
        | "io_copy_range"
        | "async_write_file"
        | "async_write_files"
        | "ssg_render_and_write_pages"
        | "ssg_read_render_and_write_pages"
        | "kv_set" => Some(NativeCapability::FilesystemWrite),

        // Filesystem delete
        "delete_file" | "os_rmdir" => Some(NativeCapability::FilesystemDelete),

        // Process execution
        "spawn_process" | "pipe_commands" => Some(NativeCapability::ProcessExec),

        // Shell execution
        "execute" | "execute_status" => Some(NativeCapability::ShellExec),

        // Environment read/write
        "env" | "env_or" | "env_int" | "env_float" | "env_bool" | "env_required" | "env_list" => {
            Some(NativeCapability::EnvRead)
        }
        "env_set" => Some(NativeCapability::EnvWrite),

        // Network client/server
        "parallel_http" | "http_get" | "http_post" | "http_request" | "http_put"
        | "http_delete" | "http_get_binary" | "http_get_stream" | "oauth2_get_token"
        | "ai_chat" | "ai_stream_chat" | "ai_embedding" | "ai_tool_loop" | "tcp_connect"
        | "tcp_send" | "tcp_receive" | "udp_send_to" | "udp_receive_from" | "async_http_get"
        | "async_http_post" => Some(NativeCapability::NetworkClient),
        "tcp_listen" | "tcp_accept" | "udp_bind" | "http_listen" => {
            Some(NativeCapability::NetworkServer)
        }

        // Database
        "db_connect" | "db_execute" | "db_query" | "db_close" | "db_pool" | "db_pool_acquire"
        | "db_pool_release" | "db_pool_stats" | "db_pool_close" | "db_begin" | "db_commit"
        | "db_rollback" | "db_last_insert_id" => Some(NativeCapability::Database),

        // Clock/time
        "now" | "now_utc" | "now_unix" | "current_timestamp" | "performance_now" | "time_us"
        | "time_ns" | "format_duration" | "elapsed" | "format_date" | "parse_date" | "sleep"
        | "async_sleep" | "async_timeout" => Some(NativeCapability::Clock),

        // Randomness
        "random" | "random_int" | "random_choice" | "uuid_v4" | "random_id" | "set_random_seed"
        | "clear_random_seed" => Some(NativeCapability::Random),

        _ => None,
    }
}
