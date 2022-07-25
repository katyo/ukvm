use crate::{Result, Server};
use dbus_async::{Binder, DBus, DBusResult};
use dbus_message_parser::{message::Message, value::Value};
use std::convert::TryInto;

#[async_trait::async_trait]
impl dbus_async::Handler for Server {
    async fn handle(&mut self, dbus: &DBus, msg: Message) -> DBusResult<()> {
        println!("Got message {:?}", msg);
        if let Ok(mut msg) = msg.method_return() {
            msg.add_value(Value::String("Hello world".to_string()));
            println!("Response: Hello world");
            dbus.send(msg)?;
        }
        Ok(())
    }
}

impl Server {
    pub async fn dbus(&self) -> Result<()> {
        let (dbus, _) = dbus_async::DBus::system(true, true).await?;

        // Create the object
        let dbus_object = DBusObject {
            property: "-".to_string(),
        };

        let object_path = "/org/example/object/path".try_into().unwrap();
        // Bind the same object to the second object path
        dbus_object.bind(dbus, object_path).await?;

        Ok(())
    }
}
