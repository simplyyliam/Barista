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
        Some("db") => db(),
        Some("db-images") => db_images(),
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

// db: read Windows' notification database (read-only) to see what the toast XML holds.
pub fn db() {
    let path = std::path::PathBuf::from(std::env::var("LOCALAPPDATA").unwrap()).join("Microsoft").join("Windows").join("Notifications").join("wpndatabase.db");
    let conn = rusqlite::Connection::open_with_flags(
        &path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    );
    let conn = match conn { Ok(c) => c, Err(e) => return println!("open error: {e}") };
    let tables: Vec<String> = conn
        .prepare("select name from sqlite_master where type='table'").unwrap()
        .query_map([], |r| r.get(0)).unwrap().flatten().collect();
    println!("tables: {tables:?}");
    for t in ["Notification", "NotificationHandler"] {
        let cols: Vec<String> = conn.prepare(&format!("pragma table_info({t})")).unwrap()
            .query_map([], |r| r.get::<_, String>(1)).unwrap().flatten().collect();
        println!("{t}: {cols:?}");
    }
}

// db-images: ids, app ids and only the image/action tags (never the text, it is private).
pub fn db_images() {
    let path = std::path::PathBuf::from(std::env::var("LOCALAPPDATA").unwrap()).join("Microsoft").join("Windows").join("Notifications").join("wpndatabase.db");
    let conn = rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let mut st = conn.prepare("select n.Id, h.PrimaryId, n.Type, n.PayloadType, n.Payload, n.ArrivalTime from Notification n join NotificationHandler h on h.RecordId = n.HandlerId order by n.Id").unwrap();
    let rows = st.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, Option<String>>(3)?, r.get::<_, Vec<u8>>(4)?, r.get::<_, i64>(5)?))).unwrap();
    let (mut n, mut with_img) = (0, 0);
    for row in rows.flatten() {
        n += 1;
        let xml = String::from_utf8_lossy(&row.4).to_string();
        let tags: Vec<&str> = xml.match_indices("<image").chain(xml.match_indices("<action")).map(|(i, _)| &xml[i..(xml[i..].find('>').map(|e| i + e + 1).unwrap_or(xml.len()))]).collect();
        if !tags.is_empty() { with_img += 1; println!("id {} app {} type {} ptype {:?} arrival {} tags {:?}", row.0, row.1, row.2, row.3, row.5, tags); }
    }
    println!("{n} rows, {with_img} with image/action tags");
}
