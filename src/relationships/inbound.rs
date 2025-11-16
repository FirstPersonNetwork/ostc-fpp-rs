/*!
*   Handles inbound relationship requests
*/

use anyhow::Result;

use crate::relationships::{Relationship, Relationships};

impl Relationship {
    /// Accepts an incoming relationship request from a remote party
    pub async fn accept_request() -> Result<()> {
        Ok(())
    }
}
