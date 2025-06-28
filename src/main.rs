use std::{
    collections::HashMap,
    ffi::OsStr,
    path::{Path, PathBuf},
};

use iced::{
    Element, Font, Size, Subscription, Task, Theme,
    widget::{Column, button, checkbox, column, container, row, scrollable},
    window::{Position, Settings, icon},
};
use rfd::{AsyncFileDialog, FileHandle};
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
            size: Size::new(720.0, 400.0),
            min_size: Some(Size::new(500.0, 310.0)),
            position: Position::Centered,
            ..Default::default()
        })
        .default_font(Font::with_name("Microsoft YaHei UI"))
        .run()
}

struct App {
    images: HashMap<PathBuf, (String, bool)>,
    theme: Theme,
    convert_img_format: HashMap<ImageFormatExt, bool>,
    select_all_images: bool,
}

impl Default for App {
    fn default() -> Self {
        App {
            images: HashMap::new(),
            theme: Theme::default(),
            convert_img_format: ImageFormatExt::get_all(),
            select_all_images: false,
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    Clear,
    ToggleImageItem(PathBuf),
    ToggleImageFormatItem(ImageFormatExt, bool),
    OpenFileDialog,
    OpenFolderDialog,
    FileSelected(Option<Vec<FileHandle>>),
    FolderSelected(Option<Vec<FileHandle>>),
    EventOccurred(iced::Event),
    SelectAllImage(bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageFormatExt {
    Png,
    Jpeg,
    WebP,
    Tiff,
    Bmp,
    Ico,
    Avif,
    Svg,
}

impl ImageFormatExt {
    fn get_all() -> HashMap<ImageFormatExt, bool> {
        let mut vec = HashMap::new();
        vec.insert(ImageFormatExt::Png, false);
        vec.insert(ImageFormatExt::Jpeg, false);
        vec.insert(ImageFormatExt::WebP, false);
        vec.insert(ImageFormatExt::Tiff, false);
        vec.insert(ImageFormatExt::Bmp, false);
        vec.insert(ImageFormatExt::Ico, false);
        vec.insert(ImageFormatExt::Avif, false);
        vec.insert(ImageFormatExt::Svg, false);
        vec
    }

    fn get_name(&self) -> &str {
        match self {
            ImageFormatExt::Png => "PNG",
            ImageFormatExt::Jpeg => "JPEG",
            ImageFormatExt::WebP => "WEBP",
            ImageFormatExt::Tiff => "TIFF",
            ImageFormatExt::Bmp => "BMP",
            ImageFormatExt::Ico => "ICO",
            ImageFormatExt::Avif => "AVIF",
            ImageFormatExt::Svg => "SVG",
        }
    }

    fn get_format(&self) -> Option<image::ImageFormat> {
        match self {
            ImageFormatExt::Png => Some(image::ImageFormat::Png),
            ImageFormatExt::Jpeg => Some(image::ImageFormat::Jpeg),
            ImageFormatExt::WebP => Some(image::ImageFormat::WebP),
            ImageFormatExt::Tiff => Some(image::ImageFormat::Tiff),
            ImageFormatExt::Bmp => Some(image::ImageFormat::Bmp),
            ImageFormatExt::Ico => None,
            ImageFormatExt::Avif => Some(image::ImageFormat::Avif),
            ImageFormatExt::Svg => None,
        }
    }
}

impl App {
    fn update(&mut self, event: Message) -> Task<Message> {
        match event {
            Message::SelectAllImage(should_select) => {
                self.select_all_images = should_select;
                self.images
                    .iter_mut()
                    .for_each(|(_, (_, c))| *c = should_select);

                Task::none()
            }
            Message::ToggleImageItem(key) => {
                if let Some(is_check) = self.images.get_mut(&key) {
                    is_check.1 = !is_check.1;
                }

                Task::none()
            }
            Message::ToggleImageFormatItem(image_format, should_convert) => {
                self.convert_img_format
                    .insert(image_format, !should_convert);

                Task::none()
            }
            Message::Clear => {
                self.images.clear();
                Task::none()
            }
            Message::FileSelected(files_handle) => {
                if let Some(files_handle) = files_handle {
                    files_handle
                        .into_iter()
                        .for_each(|file_handle| self.check_image(file_handle.path()))
                }

                Task::none()
            }
            Message::FolderSelected(folders_handle) => {
                if let Some(folders_handle) = folders_handle {
                    folders_handle.into_iter().for_each(|folder_handle| {
                        self.get_image_file_from_folder(folder_handle.path())
                    })
                }

                Task::none()
            }
            Message::OpenFileDialog => Task::perform(
                AsyncFileDialog::new().set_title("选择文件").pick_files(),
                Message::FileSelected,
            ),
            Message::OpenFolderDialog => Task::perform(
                AsyncFileDialog::new()
                    .set_title("选择文件夹")
                    .pick_folders(),
                Message::FolderSelected,
            ),
            Message::EventOccurred(event) => {
                // 文件拖拽处理
                if let iced::Event::Window(iced::window::Event::FileDropped(path)) = event {
                    if path.is_dir() {
                        self.get_image_file_from_folder(&path)
                    } else if path.is_file() {
                        self.check_image(&path);
                    }
                }

                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let select_files_button = button("选择文件")
            .on_press(Message::OpenFileDialog)
            .width(iced::Length::Fill);

        let select_folders_button = button("选择文件夹")
            .on_press(Message::OpenFolderDialog)
            .width(iced::Length::Fill);

        let clear_button = button("清空")
            .on_press(Message::Clear)
            .width(iced::Length::Fill);

        let convert_button = button("转换").width(iced::Length::Fill);

        let show_iamge_list = container(
            column![
                row![
                    select_files_button,
                    select_folders_button,
                    clear_button,
                    convert_button
                ]
                .width(iced::Length::Fill)
                .spacing(10),
                container(
                    scrollable(
                        Column::with_children(self.images.iter().enumerate().map(
                            |(index, (path, (_mime, is_checked)))| {
                                if index == 0 {
                                    checkbox("< 选 择 所 有 >", self.select_all_images)
                                        .style(checkbox::success)
                                        .on_toggle(Message::SelectAllImage)
                                        .into()
                                } else {
                                    checkbox(
                                        path.file_name()
                                            .and_then(OsStr::to_str)
                                            .unwrap_or("<未知文件名>"),
                                        *is_checked,
                                    )
                                    .on_toggle(|_| Message::ToggleImageItem(path.into()))
                                    .into()
                                }
                            }
                        ))
                        .spacing(10),
                    )
                    .width(iced::Length::Fill)
                    .height(iced::Length::Fill)
                )
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .padding(10)
                .style(container::bordered_box),
            ]
            .spacing(10),
        )
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .padding(10)
        .style(container::rounded_box);

        let show_image_format = container(
            Column::with_children(self.convert_img_format.iter().map(
                |(image_formamt, should_convert)| {
                    checkbox(image_formamt.get_name(), *should_convert)
                        .on_toggle(|_| {
                            Message::ToggleImageFormatItem(*image_formamt, *should_convert)
                        })
                        .into()
                },
            ))
            .spacing(10)
            .width(iced::Length::Fill)
            .height(iced::Length::Fill),
        )
        .width(100)
        .height(iced::Length::Fill)
        .padding(10)
        .style(container::bordered_box);

        let interface = row![show_iamge_list, show_image_format,]
            .spacing(10)
            .padding(10);

        interface.into()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::event::listen().map(Message::EventOccurred)
    }

    fn theme(&self) -> Theme {
        self.theme.clone()
    }

    fn check_image(&mut self, file_path: &Path) {
        if let Some(m) = tika_magic::from_filepath(file_path) {
            if m.starts_with("image") {
                self.images.insert(file_path.into(), (m.to_string(), false));
            }
        }
    }

    fn get_image_file_from_folder(&mut self, folder_path: &Path) {
        WalkDir::new(folder_path)
            .into_iter()
            .filter_map(|e| e.ok().filter(|e| e.file_type().is_file()))
            .for_each(|entry| {
                self.check_image(entry.path());
            });
    }
}
