use std::collections::HashMap;
use zbus::Connection;
use zbus::fdo::Result as FdoResult;
use zbus::fdo::Error as FdoError;
use zbus_polkit::policykit1::{AuthorityProxy, Subject, CheckAuthorizationFlags};

/// Security agent for Polkit authorization checks
pub struct SecurityAgent;

impl SecurityAgent {
    /// Check if the calling process is authorized for the given action.
    /// 
    /// This connects to PolicyKit on the system bus and verifies the caller's
    /// permissions against the defined policy rules.
    /// 
    /// # Arguments
    /// * `caller_pid` - PID of the calling process
    /// * `action` - Action ID like "org.speech.service.think"
    /// 
    /// # Returns
    /// * `Ok(())` if authorized
    /// * `Err(AccessDenied)` if not authorized
    pub async fn check_permission_polkit(caller_pid: u32, action: &str) -> FdoResult<()> {
        // Connect to system bus for Polkit
        let system_conn = Connection::system().await
            .map_err(|e| FdoError::Failed(format!("System bus: {}", e)))?;
        
        let proxy = AuthorityProxy::new(&system_conn).await
            .map_err(|e| FdoError::Failed(format!("Polkit proxy: {}", e)))?;
        
        // Create subject from caller's PID
        let subject = Subject::new_for_owner(caller_pid, None, None)
            .map_err(|e| FdoError::Failed(format!("Subject: {}", e)))?;
        
        // Empty details map with &str references
        let details: HashMap<&str, &str> = HashMap::new();
        
        // Check authorization with user interaction allowed
        let result = proxy.check_authorization(
            &subject,
            action,
            &details,
            CheckAuthorizationFlags::AllowUserInteraction.into(),
            "", // cancellation_id
        ).await.map_err(|e| FdoError::Failed(format!("Polkit check: {}", e)))?;
        
        if result.is_authorized {
            println!("POLKIT: Authorized '{}' for PID {}", action, caller_pid);
            Ok(())
        } else {
            println!("POLKIT: Denied '{}' for PID {}", action, caller_pid);
            Err(FdoError::AccessDenied(format!(
                "Not authorized for action '{}'", action
            )))
        }
    }
    
    /// Get the PID of a D-Bus sender by querying the bus.
    /// 
    /// # Arguments
    /// * `conn` - The session bus connection
    /// * `sender` - The D-Bus unique name (e.g., ":1.234")
    pub async fn get_sender_pid(conn: &Connection, sender: &str) -> Result<u32, FdoError> {
        use zbus::names::BusName;
        
        let dbus_proxy = zbus::fdo::DBusProxy::new(conn).await
            .map_err(|e| FdoError::Failed(format!("DBus proxy: {}", e)))?;
        
        // Convert sender string to BusName
        let bus_name = BusName::try_from(sender)
            .map_err(|e| FdoError::Failed(format!("Invalid bus name: {}", e)))?;
        
        let pid = dbus_proxy.get_connection_unix_process_id(bus_name).await
            .map_err(|e| FdoError::Failed(format!("GetConnectionUnixProcessID: {}", e)))?;
        
        Ok(pid)
    }
}
