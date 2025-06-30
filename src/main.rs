mod compress;
mod convert;

use std::{
    collections::HashMap,
    ffi::OsStr,
    path::{Path, PathBuf},
};

use iced::{
    Element, Font, Size, Subscription, Task, Theme,
    widget::{Column, button, checkbox, column, container, row, scrollable},
    window::{Settings, icon},
};
use rfd::{AsyncFileDialog, FileHandle};
use walkdir::WalkDir;

fn main() -> iced::Result {
    iced::application(App::default, App::update, App::view)
        .subscription(App::subscription)
        .theme(|_| Theme::Dark)
        .title("ImgZap")
        .window(Settings {
            icon: load_icon(),
            position: iced::window::Position::Centered,
            size: Size::new(720.0, 400.0),
            min_size: Some(Size::new(500.0, 310.0)),
            ..Default::default()
        })
        .default_font(Font::with_name("Microsoft YaHei UI"))
        .run()
}

fn load_icon() -> Option<iced::window::Icon> {
    if let Ok(image) = image::load_from_memory(include_bytes!("../assets/logo/logo.png")) {
        let image = image.to_rgba8();
        icon::from_rgba(image.to_vec(), image.width(), image.height()).ok()
    } else {
        None
    }
}

struct App {
    images: HashMap<PathBuf, (ImageFormatExt, bool)>,
    convert_img_format: HashMap<ImageFormatExt, bool>,
    select_all_images: bool,
}

impl Default for App {
    fn default() -> Self {
        App {
            images: HashMap::new(),
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
    SelectAllImage(bool),
    DropFile(PathBuf),
    ConvertImage,
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

    fn get_format_from_mime(mime: &str) -> Option<ImageFormatExt> {
        match mime {
            "image/png" => Some(ImageFormatExt::Png),
            "image/jpeg" => Some(ImageFormatExt::Jpeg),
            "image/bmp" => Some(ImageFormatExt::Bmp),
            "image/svg+xml" => Some(ImageFormatExt::Svg),
            "image/x-icon" => Some(ImageFormatExt::Ico),
            "image/vnd.microsoft.icon" => Some(ImageFormatExt::Ico),
            "image/tiff" => Some(ImageFormatExt::Tiff),
            "image/webp" => Some(ImageFormatExt::WebP),
            "image/avif" => Some(ImageFormatExt::Avif),
            _ => None,
        }
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

    fn get_ext(&self) -> String {
        Self::get_name(&self).to_lowercase()
    }

    fn get_format(&self) -> Option<image::ImageFormat> {
        match self {
            ImageFormatExt::Png => Some(image::ImageFormat::Png),
            ImageFormatExt::Jpeg => Some(image::ImageFormat::Jpeg),
            ImageFormatExt::WebP => Some(image::ImageFormat::WebP),
            ImageFormatExt::Tiff => Some(image::ImageFormat::Tiff),
            ImageFormatExt::Bmp => Some(image::ImageFormat::Bmp),
            ImageFormatExt::Avif => Some(image::ImageFormat::Avif),
            ImageFormatExt::Ico => None,
            ImageFormatExt::Svg => None,
        }
    }
}

impl App {
    fn update(&mut self, event: Message) -> Task<Message> {
        match event {
            Message::SelectAllImage(should_select) => {
                if self.select_all_images.ne(&should_select) {
                    self.select_all_images = should_select;
                    self.images
                        .iter_mut()
                        .for_each(|(_, (_, c))| *c = should_select);
                }

                Task::none()
            }
            Message::ToggleImageItem(key) => {
                if let Some((_, is_check)) = self.images.get_mut(&key) {
                    *is_check = !*is_check;
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
                self.select_all_images = false;
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
            Message::DropFile(path) => {
                if path.is_dir() {
                    self.get_image_file_from_folder(&path)
                } else if path.is_file() {
                    self.check_image(&path)
                }

                Task::none()
            }
            Message::ConvertImage => {
                convert::image_to_other(&self.images, &self.convert_img_format);

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

        let convert_button = button("转换")
            .on_press(Message::ConvertImage)
            .width(iced::Length::Fill);

        let mut images_list = Column::new()
            .push(
                checkbox("< 选 择 所 有 >", self.select_all_images)
                    .style(checkbox::success)
                    .on_toggle(Message::SelectAllImage),
            )
            .spacing(10);

        for (path, (_mime, is_checked)) in self.images.iter() {
            images_list = images_list.push(
                checkbox(
                    path.file_name()
                        .and_then(OsStr::to_str)
                        .unwrap_or("<未知文件名>"),
                    *is_checked,
                )
                .on_toggle(|_| Message::ToggleImageItem(path.into())),
            );
        }

        let show_iamges_list = container(
            column![
                row![
                    select_files_button,
                    select_folders_button,
                    clear_button,
                    convert_button
                ]
                .width(iced::Length::Fill)
                .height(30)
                .spacing(10),
                container(
                    scrollable(images_list)
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
            scrollable(
                Column::with_children(self.convert_img_format.iter().map(
                    |(image_formamt, should_convert)| {
                        checkbox(image_formamt.get_name(), *should_convert)
                            .on_toggle(|_| {
                                Message::ToggleImageFormatItem(*image_formamt, *should_convert)
                            })
                            .into()
                    },
                ))
                .spacing(10),
            )
            .width(iced::Length::Fill)
            .height(iced::Length::Fill),
        )
        .width(100)
        .height(iced::Length::Fill)
        .padding(10)
        .style(container::bordered_box);

        let interface = row![show_iamges_list, show_image_format,]
            .spacing(10)
            .padding(10);

        interface.into()
    }

    fn subscription(&self) -> Subscription<Message> {
        use iced::Event::Window;
        use iced::window::Event::FileDropped;
        iced::event::listen_with(|event, _, _| match event {
            Window(FileDropped(path)) => Some(Message::DropFile(path)),
            _ => None,
        })
    }

    fn check_image(&mut self, file_path: &Path) {
        if let Some(mime) = tika_magic::from_filepath(file_path) {
            if let Some(format) = ImageFormatExt::get_format_from_mime(mime) {
                self.images.insert(file_path.into(), (format, false));
            } else {
                println!("Not an image or image does not support conversion: \n{file_path:?}\n")
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
