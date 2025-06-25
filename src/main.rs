use std::{collections::HashMap, path::PathBuf};

use iced::{
    border::Radius, widget::{button, column, container, scrollable, text, Column}, window::{self, icon, Position, Settings}, Border, Color, Element, Font, Size, Subscription, Task, Theme
};
use rfd::FileDialog;
use walkdir::WalkDir;

fn main() -> iced::Result {
    let logo_icon = image::load_from_memory(include_bytes!("../assets/logo/logo.png"))
        .expect("Failed to load icon image")
        .to_rgba8();

    iced::application(App::default, App::update, App::view)
        .subscription(App::subscription)
        .theme(App::theme)
        .title("ImgZap")
        .window(Settings {
            icon: Some(
                icon::from_rgba(logo_icon.to_vec(), logo_icon.width(), logo_icon.height()).unwrap(),
            ),
            size: Size::new(600.0, 400.0),
            position: Position::Centered,
            ..Default::default()
        })
        .default_font(Font::with_name("Microsoft YaHei UI"))
        .run()
}

#[derive(Default)]
struct App {
    images: HashMap<PathBuf, String>,
    theme: Theme,
}

#[derive(Debug, Clone)]
enum Message {
    Clear,
    FileSelected,
    FolderSelected,
    EventOccurred(iced::Event),
    Quit,
}

impl App {
    fn update(&mut self, event: Message) -> Task<Message> {
        match event {
            Message::Clear => {
                self.images.clear();
                Task::none()
            },
            Message::FileSelected => {
                let select_files_path = FileDialog::new()
                    .set_title("选择文件")
                    .pick_files();

                if let Some(paths) = select_files_path {
                    paths.into_iter().for_each(|p| {
                        self.check_image(p);
                    });
                }

                Task::none()
            },
            Message::FolderSelected => {
                let select_folders_path = FileDialog::new()
                    .set_title("选择文件夹")
                    .pick_folders();

                if let Some(paths) = select_folders_path {
                    paths.into_iter().for_each(|path| {
                        self.get_image_file_from_folder(path)
                    });
                }

                Task::none()
            },
            Message::EventOccurred(event) => {
                if let iced::Event::Window(iced::window::Event::FileDropped(path)) = event {
                    if path.is_dir() {
                        self.get_image_file_from_folder(path)
                    } else if path.is_file() {
                        self.check_image(path);
                    }
                }

                Task::none()
            },
            Message::Quit => window::get_latest().and_then(window::close),
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let select_files_button = button("选择文件")
            .on_press(Message::FileSelected)
            .width(iced::Length::Fill);

        let select_folders_button = button("选择文件夹")
            .on_press(Message::FolderSelected)
            .width(iced::Length::Fill);

        let clear_button = button("清空")
            .on_press(Message::Clear)
            .width(iced::Length::Fill);

        let quit_button = button("退出")
            .on_press(Message::Quit)
            .width(iced::Length::Fill);

        let show_iamge_info = text(format!(
            "图片数量: {}\n{}",
            self.images.len(),
            self.images
                .iter()
                .map(|(p, f)| format!("{f:?}: {p:?}"))
                .collect::<Vec<_>>()
                .join("\n")
        )).wrapping(text::Wrapping::None);

        let interface = column![
            select_files_button,
            select_folders_button,
            clear_button,
            quit_button,
        ]
            .spacing(20)
            .padding(20)
            .width(iced::Length::Fill);

        if self.images.len() > 0 {
            interface.push(
                container(
                    scrollable(show_iamge_info)
                        .width(iced::Length::Fill)
                        .height(iced::Length::Fill)
                )
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .padding(10)
                .style(container::bordered_box)
                
            ).into()    
        } else {
            interface.into()
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::event::listen().map(Message::EventOccurred)
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn check_image(&mut self, file_path: PathBuf) {
        if let Some(m) = tika_magic::from_filepath(&file_path) {
            if m.starts_with("image") {
                self.images.insert(file_path, m.to_string());
            } else {
                println!("\nNot an image file: {m}\n{file_path:?}\n");
            }
        }
    }

    fn get_image_file_from_folder(&mut self, folder_path: PathBuf) {
        WalkDir::new(folder_path)
            .into_iter()
            .filter_map(|e| e.ok().filter(|e| e.file_type().is_file()))
            .for_each(|entry| {
                self.check_image(entry.into_path());
            });
    }
}