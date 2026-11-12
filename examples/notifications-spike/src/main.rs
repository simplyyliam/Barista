// Spike: what can an unpackaged desktop app read through UserNotificationListener?
// Usage: notifications-spike access | list | watch [secs] | dismiss <id> | clear
use std::time::Duration;
use windows::{
    Foundation::{Size, TypedEventHandler},
    UI::Notifications::{
        KnownNotificationBindings, Management::UserNotificationListener,
        NotificationKinds, UserNotification, UserNotificationChangedEventArgs,
    },
    Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED},
};

type R<T> = windows::core::Result<T>;

fn describe(n: &UserNotification) -> R<()> {
    println!("- id {} created {:?}", n.Id()?, n.CreationTime()?);
    match n.AppInfo() {
        Ok(app) => {
            let info = app.DisplayInfo()?;
            println!("  app: {} ({})", info.DisplayName()?, app.AppUserModelId()?);
            match info.GetLogo(Size { Width: 64.0, Height: 64.0 }) {
                Err(e) => println!("  logo: GetLogo error 0x{:08X}", e.code().0),
                Ok(r) => match r.OpenReadAsync().and_then(|o| o.join()) {
                    Ok(s) => println!("  logo: {} bytes ({})", s.Size()?, s.ContentType()?),
                    Err(e) => println!("  logo: open error 0x{:08X} {}", e.code().0, e.message()),
                },
            }
        }
        Err(e) => println!("  app info: error {e}"),
    }
    let toast = n.Notification()?;
    println!("  expires: {:?}", toast.ExpirationTime().ok());
    let visual = toast.Visual()?;
    for b in visual.Bindings()? {
        println!("  binding template: {}", b.Template()?);
    }
    let generic = visual.GetBinding(&KnownNotificationBindings::ToastGeneric()?)?;
    for (i, t) in generic.GetTextElements()?.into_iter().enumerate() {
        println!("  text[{i}]: {}", t.Text()?);
    }
    Ok(())
}

fn list(l: &UserNotificationListener) -> R<()> {
    let all = l.GetNotificationsAsync(NotificationKinds::Toast)?.join()?;
    println!("{} toast(s)", all.Size()?);
    for n in all {
        describe(&n)?;
    }
    Ok(())
}

fn run() -> R<()> {
    unsafe { RoInitialize(RO_INIT_MULTITHREADED)? };
    let args: Vec<String> = std::env::args().collect();
    let l = UserNotificationListener::Current()?;
    match args.get(1).map(String::as_str) {
        Some("access") => {
            println!("status before: {:?}", l.GetAccessStatus()?);
            println!("request: {:?}", l.RequestAccessAsync()?.join());
            println!("status after: {:?}", l.GetAccessStatus()?);
        }
        Some("list") => list(&l)?,
        Some("watch") => {
            let secs: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(60);
            let token = l.NotificationChanged(&TypedEventHandler::new(
                |_: windows::core::Ref<UserNotificationListener>,
                 a: windows::core::Ref<UserNotificationChangedEventArgs>| {
                    if let Some(a) = a.as_ref() {
                        println!("changed: {:?} id {:?}", a.ChangeKind(), a.UserNotificationId());
                    }
                    Ok(())
                },
            ))?;
            println!("watching {secs}s (send toasts now)");
            std::thread::sleep(Duration::from_secs(secs));
            l.RemoveNotificationChanged(token)?;
        }
        Some("dismiss") => {
            let id: u32 = args.get(2).and_then(|s| s.parse().ok()).expect("dismiss <id>");
            l.RemoveNotification(id)?;
            println!("removed {id}");
        }
        Some("clear") => {
            l.ClearNotifications()?;
            println!("cleared");
        }
        _ => println!("usage: access | list | watch [secs] | dismiss <id> | clear"),
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        println!("ERROR 0x{:08X}: {}", e.code().0, e.message());
    }
}
