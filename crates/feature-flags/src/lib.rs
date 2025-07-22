use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use wasm_storage::simple::SimpleStorage;

/// Feature flag configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlag {
    /// Unique identifier for the feature
    pub id: String,
    
    /// Human-readable name
    pub name: String,
    
    /// Description of the feature
    pub description: String,
    
    /// Whether the feature is enabled
    pub enabled: bool,
    
    /// Rollout percentage (0-100)
    pub rollout_percentage: u8,
    
    /// List of user IDs that have access
    pub allowed_users: Vec<String>,
    
    /// List of user IDs that are blocked
    pub blocked_users: Vec<String>,
    
    /// Feature dependencies
    pub dependencies: Vec<String>,
    
    /// Metadata
    pub metadata: HashMap<String, String>,
}

/// Feature flag manager
#[wasm_bindgen]
pub struct FeatureFlagManager {
    flags: HashMap<String, FeatureFlag>,
    storage: SimpleStorage,
    storage_key: String,
    user_id: Option<String>,
}

#[wasm_bindgen]
impl FeatureFlagManager {
    /// Create a new feature flag manager
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let storage = SimpleStorage::local();
        let storage_key = "gpu_charts_feature_flags".to_string();
        
        // Load flags from storage
        let flags = storage
            .get_json::<HashMap<String, FeatureFlag>>(&storage_key)
            .ok()
            .flatten()
            .unwrap_or_default();
        
        Self {
            flags,
            storage,
            storage_key,
            user_id: None,
        }
    }
    
    /// Set the current user ID for personalized feature flags
    pub fn set_user_id(&mut self, user_id: Option<String>) {
        self.user_id = user_id;
    }
    
    /// Check if a feature is enabled
    pub fn is_enabled(&self, feature_id: &str) -> bool {
        if let Some(flag) = self.flags.get(feature_id) {
            // Check if all dependencies are enabled
            if !self.check_dependencies(&flag.dependencies) {
                return false;
            }
            
            // Check user-specific rules
            if let Some(ref user_id) = self.user_id {
                // Blocked users
                if flag.blocked_users.contains(user_id) {
                    return false;
                }
                
                // Allowed users
                if !flag.allowed_users.is_empty() && flag.allowed_users.contains(user_id) {
                    return true;
                }
            }
            
            // Check rollout percentage
            if flag.rollout_percentage < 100 {
                // Use user ID for consistent rollout
                if let Some(ref user_id) = self.user_id {
                    let hash = self.hash_user_id(user_id, feature_id);
                    let percentage = (hash % 100) as u8;
                    return flag.enabled && percentage < flag.rollout_percentage;
                }
            }
            
            flag.enabled
        } else {
            false
        }
    }
    
    /// Register a new feature flag
    pub fn register_flag(&mut self, flag_json: &str) -> Result<(), JsValue> {
        let flag: FeatureFlag = serde_json::from_str(flag_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse flag: {}", e)))?;
        
        self.flags.insert(flag.id.clone(), flag);
        self.save_to_storage()?;
        
        Ok(())
    }
    
    /// Update a feature flag
    pub fn update_flag(&mut self, feature_id: &str, updates_json: &str) -> Result<(), JsValue> {
        let updates: HashMap<String, serde_json::Value> = serde_json::from_str(updates_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse updates: {}", e)))?;
        
        if let Some(flag) = self.flags.get_mut(feature_id) {
            // Apply updates
            if let Some(enabled) = updates.get("enabled").and_then(|v| v.as_bool()) {
                flag.enabled = enabled;
            }
            if let Some(percentage) = updates.get("rollout_percentage").and_then(|v| v.as_u64()) {
                flag.rollout_percentage = percentage.min(100) as u8;
            }
            if let Some(allowed) = updates.get("allowed_users").and_then(|v| v.as_array()) {
                flag.allowed_users = allowed
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
            }
            if let Some(blocked) = updates.get("blocked_users").and_then(|v| v.as_array()) {
                flag.blocked_users = blocked
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
            }
            
            self.save_to_storage()?;
            Ok(())
        } else {
            Err(JsValue::from_str("Feature flag not found"))
        }
    }
    
    /// Get all feature flags
    pub fn get_all_flags(&self) -> String {
        serde_json::to_string(&self.flags).unwrap_or_default()
    }
    
    /// Get a specific feature flag
    pub fn get_flag(&self, feature_id: &str) -> Option<String> {
        self.flags.get(feature_id)
            .and_then(|flag| serde_json::to_string(flag).ok())
    }
    
    /// Remove a feature flag
    pub fn remove_flag(&mut self, feature_id: &str) -> Result<(), JsValue> {
        self.flags.remove(feature_id);
        self.save_to_storage()?;
        Ok(())
    }
    
    /// Get enabled features for the current user
    pub fn get_enabled_features(&self) -> Vec<JsValue> {
        self.flags
            .keys()
            .filter(|id| self.is_enabled(id))
            .map(|id| JsValue::from_str(id))
            .collect()
    }
}

impl FeatureFlagManager {
    /// Check if all dependencies are enabled
    fn check_dependencies(&self, dependencies: &[String]) -> bool {
        dependencies.iter().all(|dep| {
            self.flags.get(dep).map(|flag| flag.enabled).unwrap_or(false)
        })
    }
    
    /// Hash user ID for consistent rollout
    fn hash_user_id(&self, user_id: &str, feature_id: &str) -> u32 {
        let combined = format!("{}{}", user_id, feature_id);
        let mut hash = 0u32;
        for byte in combined.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u32);
        }
        hash
    }
    
    /// Save flags to storage
    fn save_to_storage(&self) -> Result<(), JsValue> {
        self.storage.set_json(&self.storage_key, &self.flags)?;
        Ok(())
    }
}

/// Default feature flags for the charting library
impl Default for FeatureFlagManager {
    fn default() -> Self {
        let mut manager = Self::new();
        
        // Register default feature flags
        let default_flags = vec![
            FeatureFlag {
                id: "binary_culling".to_string(),
                name: "Binary Culling".to_string(),
                description: "GPU-accelerated binary search culling".to_string(),
                enabled: true,
                rollout_percentage: 100,
                allowed_users: vec![],
                blocked_users: vec![],
                dependencies: vec![],
                metadata: HashMap::new(),
            },
            FeatureFlag {
                id: "vertex_compression".to_string(),
                name: "Vertex Compression".to_string(),
                description: "Compress vertex data for reduced memory usage".to_string(),
                enabled: true,
                rollout_percentage: 100,
                allowed_users: vec![],
                blocked_users: vec![],
                dependencies: vec![],
                metadata: HashMap::new(),
            },
            FeatureFlag {
                id: "gpu_vertex_generation".to_string(),
                name: "GPU Vertex Generation".to_string(),
                description: "Generate vertices on GPU for better performance".to_string(),
                enabled: true,
                rollout_percentage: 100,
                allowed_users: vec![],
                blocked_users: vec![],
                dependencies: vec![],
                metadata: HashMap::new(),
            },
            FeatureFlag {
                id: "render_bundles".to_string(),
                name: "Render Bundles".to_string(),
                description: "Experimental render bundle optimization".to_string(),
                enabled: false,
                rollout_percentage: 0,
                allowed_users: vec![],
                blocked_users: vec![],
                dependencies: vec![],
                metadata: HashMap::new(),
            },
            FeatureFlag {
                id: "websocket_data".to_string(),
                name: "WebSocket Data Feed".to_string(),
                description: "Real-time data via WebSocket connections".to_string(),
                enabled: false,
                rollout_percentage: 50,
                allowed_users: vec![],
                blocked_users: vec![],
                dependencies: vec![],
                metadata: HashMap::new(),
            },
            FeatureFlag {
                id: "advanced_charts".to_string(),
                name: "Advanced Chart Types".to_string(),
                description: "Scatter plots, heatmaps, and 3D charts".to_string(),
                enabled: false,
                rollout_percentage: 0,
                allowed_users: vec![],
                blocked_users: vec![],
                dependencies: vec!["gpu_vertex_generation".to_string()],
                metadata: HashMap::new(),
            },
            FeatureFlag {
                id: "technical_indicators".to_string(),
                name: "Technical Indicators".to_string(),
                description: "SMA, EMA, RSI, and other indicators".to_string(),
                enabled: false,
                rollout_percentage: 25,
                allowed_users: vec![],
                blocked_users: vec![],
                dependencies: vec![],
                metadata: HashMap::new(),
            },
        ];
        
        for flag in default_flags {
            manager.flags.insert(flag.id.clone(), flag);
        }
        
        // Save to storage
        let _ = manager.save_to_storage();
        
        manager
    }
}