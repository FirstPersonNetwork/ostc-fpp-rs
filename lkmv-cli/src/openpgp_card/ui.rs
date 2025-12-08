/*! Common UI Functions dealing with openpgp-cards
*/
use dialoguer::{Password, theme::ColorfulTheme};
use secrecy::SecretString;
use std::fmt;

#[derive(Default)]
pub struct AdminPin(Option<SecretString>);

impl AdminPin {
    pub fn get_pin(&mut self) -> &SecretString {
        if self.0.is_none() {
            self.0 = Some(AdminPin::get_admin_pin());
        }

        self.0.as_ref().unwrap()
    }

    /// Requests Admin Pin from the user
    fn get_admin_pin() -> SecretString {
        let pin = Password::with_theme(&ColorfulTheme::default())
            .with_prompt("Please enter Token Admin PIN <blank = default>")
            .allow_empty_password(true)
            .interact()
            .unwrap();
        if pin.is_empty() {
            SecretString::new("12345678".to_string())
        } else {
            SecretString::new(pin)
        }
    }
}

impl fmt::Debug for AdminPin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AdminPin")
            .field("exists", &self.0.is_some())
            .finish()
    }
}

#[derive(Default)]
pub struct UserPin(Option<SecretString>);

impl UserPin {
    pub fn get_pin(&mut self) -> &SecretString {
        if self.0.is_none() {
            self.0 = Some(UserPin::get_user_pin());
        }

        self.0.as_ref().unwrap()
    }
    /// Requests user Pin from the user
    fn get_user_pin() -> SecretString {
        let pin = Password::with_theme(&ColorfulTheme::default())
            .with_prompt("Please enter Token User PIN <blank = default>")
            .allow_empty_password(true)
            .interact()
            .unwrap();
        if pin.is_empty() {
            SecretString::new("123456".to_string())
        } else {
            SecretString::new(pin)
        }
    }
}

impl fmt::Debug for UserPin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UserPin")
            .field("exists", &self.0.is_some())
            .finish()
    }
}
